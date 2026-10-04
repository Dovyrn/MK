#include "mk.slangh"

using namespace linalg;

RWStructuredBuffer<half> a : STORAGE(0);
RWStructuredBuffer<half> b : STORAGE(1);
RWStructuredBuffer<float> c : STORAGE(2);

typealias A = CoopMat<half, MemoryScope.Subgroup, 16, 16, CoopMatMatrixUse.MatrixA>;
typealias B = CoopMat<half, MemoryScope.Subgroup, 16, 16, CoopMatMatrixUse.MatrixB>;
typealias C = CoopMat<float, MemoryScope.Subgroup, 16, 16, CoopMatMatrixUse.MatrixAccumulator>;

[shader("compute")]
[numthreads(32, 1, 1)]
void multiply() {
    let x = A.Load<CoopMatMatrixLayout.RowMajor>(a, 0, 16);
    let y = B.Load<CoopMatMatrixLayout.RowMajor>(b, 0, 16);
    let z = coopMatMulAdd<float, false>(x, y, C(0.0));
    z.Store<CoopMatMatrixLayout.RowMajor>(c, 0, 16);
}
