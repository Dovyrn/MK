#!/bin/sh
set -e
cd "$(dirname "$0")"
mkdir -p out
for src in *.slang; do
    name="${src%.slang}"
    if [ -f "$name.glsl" ]; then
        glslangValidator -V "$name.glsl" -o "out/$name.spv"
    else
        slangc "$src" -target spirv -emit-spirv-directly -o "out/$name.spv"
    fi
    if [ -f "$name.metal" ]; then
        cp "$name.metal" "out/$name.metal"
    else
        slangc "$src" -target metal -o "out/$name.metal"
    fi
    xcrun -sdk macosx metal -std=metal4.0 -c "out/$name.metal" -o "out/$name.air"
    xcrun -sdk macosx metallib "out/$name.air" -o "out/$name.metallib"
    rm "out/$name.air"
done
