#!/bin/zsh
cd ${0:A:h}/..
O=$PWD/target/mk.out
mkdir -p target
icds=(${=MK_ICDS:-/usr/local/share/vulkan/icd.d/libkosmickrisp_icd.json $HOME/.local/mesa/share/vulkan/icd.d/lvp_icd.aarch64.json})
examples=(${=1:-compute texture render mesh view adopt queries raytrace heap queues native indirect sparse compact viewports slots handles uploads blits images})
cargo fmt --check || echo "fmt dirty"
cargo run -q -p shade
for f in mtl vk gl mtl,jni vk,jni gl,jni; do
    cargo clippy -q --features $f --all-targets -- -D warnings 2>&1 | grep -E '^(error|warning)' -A6 | head
    cargo test -q --features $f 2>&1 | grep -E 'FAIL|panicked'
done
echo "== metal"
for e in $examples; do
    MTL_DEBUG_LAYER=1 MTL_SHADER_VALIDATION=1 cargo run -q --features mtl --example $e > $O 2>&1
    echo "$e: $(tail -1 $O)"
done
cargo build -q --features vk --examples
for d in $icds; do
    echo "== ${d:t}"
    for e in $examples; do
        DYLD_FALLBACK_LIBRARY_PATH=/usr/local/lib VK_LOADER_LAYERS_ENABLE='*validation' VK_LAYER_VALIDATE_SYNC=1 VK_DRIVER_FILES=$d ./target/debug/examples/$e > $O 2>&1
        echo "$e: $(grep -ciE 'validation error|panicked' $O) errors, $(tail -1 $O)"
    done
done
cargo build -q --features gl --examples
for h in mesa apple legacy; do
    echo "== gl $h"
    for e in slots render viewports adopt view queries native mesh cache uploads blits images handles indirect; do
        MK_HOST=$h DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib ./target/debug/examples/$e > $O 2>&1
        echo "$e: $(tail -1 $O)"
    done
done
echo "== gl zink"
for e in images; do
    GALLIUM_DRIVER=zink LIBGL_ALWAYS_SOFTWARE=1 VK_DRIVER_FILES=/opt/homebrew/share/vulkan/icd.d/lvp_icd.aarch64.json DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib ./target/debug/examples/$e > $O 2>&1
    echo "$e: $(tail -1 $O)"
done
G=~/.gradle/caches/modules-2/files-2.1
jar() {
    ls $G/$1/$2/3.3.6/*/$2-3.3.6$3.jar
}
CP=$(jar org.lwjgl lwjgl):$(jar org.lwjgl lwjgl -natives-macos-arm64):$(jar org.lwjgl lwjgl-vulkan):$(jar org.lwjgl lwjgl-glfw):$(jar org.lwjgl lwjgl-glfw -natives-macos-arm64)
J=$(/usr/libexec/java_home -v 25)/bin
for l in mtl vk gl; do
    cargo build -q --manifest-path libs/$l/Cargo.toml --target-dir target
done
rm -rf target/mkjava && mkdir target/mkjava
$J/javac -proc:full -cp $CP:$(ls $G/org.projectlombok/lombok/1.18.42/*/lombok-1.18.42.jar) -d target/mkjava java/src/main/java/dev/dov/mk/*.java java/src/test/java/dev/dov/mk/*.java 2>&1 | grep -v WARNING
run() {
    $J/java --enable-native-access=ALL-UNNAMED -cp target/mkjava:$CP "$@"
}
echo "== java metal"
run dev.dov.mk.Smoke target/debug/libmk_mtl.dylib examples/shaders/out/count.metallib target/java-mtl.bin 2>&1 | grep -v WARNING | tail -1
cargo run -q --features mtl --example parity target/java-mtl.bin 2>&1 | tail -1
for d in $icds; do
    echo "== java ${d:t}"
    DYLD_FALLBACK_LIBRARY_PATH=/usr/local/lib VK_LOADER_LAYERS_ENABLE='*validation' VK_DRIVER_FILES=$d run dev.dov.mk.Smoke target/debug/libmk_vk.dylib examples/shaders/out/count.spv target/java-vk.bin > $O 2>&1
    echo "$(grep -ciE 'validation error|exception' $O) errors, $(tail -1 $O)"
done
cargo run -q --features vk --example parity target/java-vk.bin 2>&1 | tail -1
echo "== java gl"
run -XstartOnFirstThread dev.dov.mk.Smoke target/debug/libmk_gl.dylib examples/shaders/out/count.gl target/java-gl.bin 2>&1 | grep -v WARNING | tail -1
cargo run -q --features gl --example parity target/java-gl.bin 2>&1 | tail -1
