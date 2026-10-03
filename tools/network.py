import numpy as np
import coremltools as ct
from coremltools.converters.mil import Builder as mb
from coremltools.converters.mil.mil import types

n = 16
weight = np.array([[(i * 3 + j * 5) % 7 - 3 for j in range(n)] for i in range(n)], dtype=np.float32)
bias = np.array([i % 4 for i in range(n)], dtype=np.float32)


@mb.program(input_specs=[mb.TensorSpec(shape=(1, n), dtype=types.fp32)], opset_version=ct.target.macOS26)
def network(x):
    return mb.relu(x=mb.linear(x=x, weight=weight, bias=bias), name="y")


model = ct.convert(
    network,
    convert_to="mlprogram",
    compute_precision=ct.precision.FLOAT32,
    minimum_deployment_target=ct.target.macOS26,
)
model.save("shaders/network.mlpackage")
