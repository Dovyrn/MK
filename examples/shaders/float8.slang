#include "mk.slangh"

using namespace linalg;

RWStructuredBuffer<float> a : STORAGE(0);
RWStructuredBuffer<float> b : STORAGE(1);
RWStructuredBuffer<float> c : STORAGE(2);

typealias A = CoopMat<float, MemoryScope.Subgroup, 8, 8, CoopMatMatrixUse.MatrixA>;
typealias B = CoopMat<float, MemoryScope.Subgroup, 8, 8, CoopMatMatrixUse.MatrixB>;
typealias C = CoopMat<float, MemoryScope.Subgroup, 8, 8, CoopMatMatrixUse.MatrixAccumulator>;

[shader("compute")]
[numthreads(32, 1, 1)]
void multiply() {
    let x = A.Load<CoopMatMatrixLayout.RowMajor>(a, 0, 8);
    let y = B.Load<CoopMatMatrixLayout.RowMajor>(b, 0, 8);
    let z = coopMatMulAdd<float, false>(x, y, C(0.0));
    z.Store<CoopMatMatrixLayout.RowMajor>(c, 0, 8);
}
