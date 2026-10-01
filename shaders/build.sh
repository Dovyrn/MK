#!/bin/sh
set -e
cd "$(dirname "$0")"
mkdir -p out
for src in *.slang; do
    name="${src%.slang}"
    if [ -f "$name.glsl" ]; then
        glslangValidator -V "$name.glsl" -o "out/$name.spv"
    else
        slangc "$src" -target spirv -emit-spirv-directly -fvk-use-entrypoint-name -fvk-invert-y -o "out/$name.spv"
    fi
    if [ -f "$name.metal" ]; then
        xcrun -sdk macosx metal -std=metal4.0 -c "$name.metal" -o "out/$name.air"
        xcrun -sdk macosx metallib "out/$name.air" -o "out/$name.metallib"
        rm "out/$name.air"
        continue
    fi
    entries=$(awk '/\[shader\(/{s=1;next} s&&!/^\[/{match($0,/[A-Za-z_][A-Za-z0-9_]*\(/); print substr($0,RSTART,RLENGTH-1); s=0}' "$src")
    airs=""
    for entry in $entries; do
        slangc "$src" -target metal -entry "$entry" -o "out/$name.$entry.metal"
        xcrun -sdk macosx metal -std=metal4.0 -c "out/$name.$entry.metal" -o "out/$name.$entry.air"
        airs="$airs out/$name.$entry.air"
    done
    xcrun -sdk macosx metallib $airs -o "out/$name.metallib"
    rm $airs
done
