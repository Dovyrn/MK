Set-Location (Join-Path $PSScriptRoot "..")
$examples = if ($args.Count) { $args } else {
    @("compute", "texture", "render", "mesh", "view", "adopt", "queries", "raytrace", "heap", "queues", "native",
        "indirect", "sparse", "compact", "viewports", "slots", "handles", "uploads", "blits", "images", "resolve")
}
$gl = @("slots", "render", "viewports", "adopt", "view", "queries", "native", "mesh", "cache", "uploads", "blits",
    "images", "handles", "indirect", "resolve")
$quick = [bool]$env:QUICK
$configs = @("vk", "gl")
if (-not $quick) {
    $configs += @("vk,jni", "gl,jni")
}
New-Item -ItemType Directory -Force target\out | Out-Null

function Each($tag, $list, $count) {
    if ($PSVersionTable.PSVersion.Major -ge 7) {
        $list | ForEach-Object -ThrottleLimit 8 -Parallel {
            & ".\target\debug\examples\$_.exe" *> "target\out\$($using:tag).$_"
        }
    } else {
        foreach ($e in $list) {
            & ".\target\debug\examples\$e.exe" *> "target\out\$tag.$e"
        }
    }
    foreach ($e in $list) {
        $o = @(Get-Content "target\out\$tag.$e" -ErrorAction SilentlyContinue)
        $last = if ($o.Count) { $o[-1] } else { "" }
        if ($count) {
            $n = @($o | Select-String -Pattern "validation error|panicked").Count
            "${e}: $n errors, $last"
        } else {
            "${e}: $last"
        }
    }
}

cargo fmt --check
if ($LASTEXITCODE) {
    "fmt dirty"
}
cargo run -q -p compiler
foreach ($f in $configs) {
    cargo clippy -q --features $f --all-targets -- -D warnings 2>&1 | Select-String "^(error|warning)" -Context 0, 6 |
        Select-Object -First 10
    cargo test -q --features $f 2>&1 | Select-String "FAIL|panicked"
}

cargo build -q --features vk --examples
if (-not $quick) {
    $env:VK_LOADER_LAYERS_ENABLE = "*validation"
}
"== vulkan"
Each "vulkan" $examples $true
Remove-Item Env:\VK_LOADER_LAYERS_ENABLE -ErrorAction SilentlyContinue

cargo build -q --features gl --examples
foreach ($h in @("default", "legacy")) {
    "== gl $h"
    $env:MK_HOST = $h
    Each "gl-$h" $gl $false
}
Remove-Item Env:\MK_HOST -ErrorAction SilentlyContinue

if ($quick) {
    exit 0
}
$cache = "$env:USERPROFILE\.gradle\caches\modules-2\files-2.1"
function Jar($group, $name, $suffix) {
    Get-ChildItem "$cache\$group\$name\3.3.6" -Recurse -Filter "$name-3.3.6$suffix.jar" -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
}
$jars = @(
    (Jar "org.lwjgl" "lwjgl" ""),
    (Jar "org.lwjgl" "lwjgl" "-natives-windows"),
    (Jar "org.lwjgl" "lwjgl-vulkan" ""),
    (Jar "org.lwjgl" "lwjgl-glfw" ""),
    (Jar "org.lwjgl" "lwjgl-glfw" "-natives-windows")
)
$lombok = Get-ChildItem "$cache\org.projectlombok\lombok\1.18.42" -Recurse -Filter "lombok-1.18.42.jar" `
    -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if ($jars -contains $null -or -not $lombok -or -not $env:JAVA_HOME) {
    "== java skipped: LWJGL 3.3.6 and Lombok 1.18.42 must be in the Gradle cache and JAVA_HOME set"
    exit 0
}
$cp = $jars -join ";"
foreach ($l in @("vk", "gl")) {
    cargo build -q --manifest-path "libs\$l\Cargo.toml" --target-dir target
}
Remove-Item -Recurse -Force target\mkjava -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force target\mkjava | Out-Null
$sources = @(Get-ChildItem java\src\main\java\dev\dov\mk\*.java, java\src\test\java\dev\dov\mk\*.java |
    ForEach-Object FullName)
& "$env:JAVA_HOME\bin\javac" -proc:full -cp "$cp;$lombok" -d target\mkjava $sources 2>&1 | Select-String -NotMatch "WARNING"
foreach ($l in @("vk", "gl")) {
    "== java $l"
    $code = if ($l -eq "vk") { "count.spv" } else { "count.gl" }
    & "$env:JAVA_HOME\bin\java" --enable-native-access=ALL-UNNAMED -cp "target\mkjava;$cp" dev.dov.mk.Smoke `
        "target\debug\mk_$l.dll" "examples\shaders\out\$code" "target\java-$l.bin" 2>&1 |
        Select-String -NotMatch "WARNING" | Select-Object -Last 1
    cargo run -q --features $l --example parity "target\java-$l.bin" 2>&1 | Select-Object -Last 1
}
