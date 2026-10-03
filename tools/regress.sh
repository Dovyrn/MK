#!/bin/zsh
cd ${0:A:h}/..
O=$PWD/target/mk.out
mkdir -p target
icds=(${=MK_ICDS:-/usr/local/share/vulkan/icd.d/libkosmickrisp_icd.json $HOME/.local/mesa/share/vulkan/icd.d/lvp_icd.aarch64.json})
examples=(${=1:-compute texture render mesh view adopt queries raytrace heap queues multiview tile native indirect generated rays functions sparse ratemap host shared surfaces boxes compact viewports predicates slots handles draws uploads matmul network blits})
cargo fmt --check || echo "fmt dirty"
cargo run -q -p shade
for f in mtl vk gl mtl,jni vk,jni gl,jni; do
    cargo clippy -q --features $f --all-targets -- -D warnings 2>&1 | grep -E '^(error|warning)' -A6 | head
    cargo test -q --features $f 2>&1 | grep -E 'FAIL|panicked'
done
echo "== metal"
for e in $examples; do
    v=1
    [[ $e == draws ]] && v=0
    MTL_DEBUG_LAYER=1 MTL_SHADER_VALIDATION=$v cargo run -q --features mtl --example $e > $O 2>&1
    echo "$e: $(tail -1 $O)"
done
echo "== metal without shader validation, which faults inside MPP"
for e in tensor; do
    MTL_DEBUG_LAYER=1 cargo run -q --features mtl --example $e > $O 2>&1
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
echo "== lavapipe 256-bit"
LP_NATIVE_VECTOR_WIDTH=256 DYLD_FALLBACK_LIBRARY_PATH=/usr/local/lib VK_LOADER_LAYERS_ENABLE='*validation' VK_LAYER_VALIDATE_SYNC=1 VK_DRIVER_FILES=$HOME/.local/mesa/share/vulkan/icd.d/lvp_icd.aarch64.json ./target/debug/examples/matmul > $O 2>&1
echo "matmul: $(grep -ciE 'validation error|panicked' $O) errors, $(tail -1 $O)"
cargo build -q --features gl --examples
for h in mesa apple legacy; do
    echo "== gl $h"
    for e in slots render viewports adopt view queries native mesh cache draws uploads blits; do
        MK_HOST=$h DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib ./target/debug/examples/$e > $O 2>&1
        echo "$e: $(tail -1 $O)"
    done
done
G=~/.gradle/caches/modules-2/files-2.1
jar() {
    ls $G/$1/$2/3.3.6/*/$2-3.3.6$3.jar
}
CP=$(jar org.lwjgl lwjgl):$(jar org.lwjgl lwjgl -natives-macos-arm64):$(jar org.lwjgl lwjgl-vulkan):$(jar org.lwjgl lwjgl-glfw):$(jar org.lwjgl lwjgl-glfw -natives-macos-arm64)
J=$(/usr/libexec/java_home -v 25)/bin
cargo build -q -p mk-mtl && cargo build -q -p mk-vk && cargo build -q -p mk-gl
rm -rf target/mkjava && mkdir target/mkjava
$J/javac -proc:full -cp $CP:$(ls $G/org.projectlombok/lombok/1.18.42/*/lombok-1.18.42.jar) -d target/mkjava java/src/main/java/dev/dov/mk/*.java java/src/test/java/dev/dov/mk/*.java 2>&1 | grep -v WARNING
run() {
    $J/java --enable-native-access=ALL-UNNAMED -cp target/mkjava:$CP "$@"
}
echo "== java metal"
run dev.dov.mk.Smoke target/debug/libmk_mtl.dylib shaders/out/count.metallib target/java-mtl.bin 2>&1 | grep -v WARNING | tail -1
cargo run -q --features mtl --example parity target/java-mtl.bin 2>&1 | tail -1
for d in $icds; do
    echo "== java ${d:t}"
    DYLD_FALLBACK_LIBRARY_PATH=/usr/local/lib VK_LOADER_LAYERS_ENABLE='*validation' VK_DRIVER_FILES=$d run dev.dov.mk.Smoke target/debug/libmk_vk.dylib shaders/out/count.spv target/java-vk.bin > $O 2>&1
    echo "$(grep -ciE 'validation error|exception' $O) errors, $(tail -1 $O)"
done
cargo run -q --features vk --example parity target/java-vk.bin 2>&1 | tail -1
echo "== java gl"
run -XstartOnFirstThread dev.dov.mk.Smoke target/debug/libmk_gl.dylib shaders/out/count.gl target/java-gl.bin 2>&1 | grep -v WARNING | tail -1
cargo run -q --features gl --example parity target/java-gl.bin 2>&1 | tail -1
