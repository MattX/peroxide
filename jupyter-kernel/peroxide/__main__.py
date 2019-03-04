from ipykernel.kernelapp import IPKernelApp
from . import PeroxidePexpectKernel

IPKernelApp.launch_instance(kernel_class=PeroxidePexpectKernel)
