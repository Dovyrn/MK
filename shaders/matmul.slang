using namespace linalg;

struct Push {
    half* a;
    half* b;
    float* c;
    uint size;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

typealias Left = CoopMat<half, MemoryScope.Subgroup, 8, 8, CoopMatMatrixUse.MatrixA>;
typealias Right = CoopMat<half, MemoryScope.Subgroup, 8, 8, CoopMatMatrixUse.MatrixB>;
typealias Sum = CoopMat<float, MemoryScope.Subgroup, 8, 8, CoopMatMatrixUse.MatrixAccumulator>;

[shader("compute")]
[numthreads(32, 1, 1)]
void multiply(uint3 group: SV_GroupID, uint local: SV_GroupIndex) {
    if (local >= WaveGetLaneCount()) {
        return;
    }
    uint row = group.y * 8;
    uint column = group.x * 8;
    var sum = Sum(0.0);
    for (uint k = 0; k < push.size; k += 8) {
        let a = Left.Load<CoopMatMatrixLayout.RowMajor>(push.a, row * push.size + k, push.size);
        let b = Right.Load<CoopMatMatrixLayout.RowMajor>(push.b, k * push.size + column, push.size);
        sum = coopMatMulAdd<float, false>(a, b, sum);
    }
    sum.Store<CoopMatMatrixLayout.RowMajor>(push.c, row * push.size + column, push.size);
}
