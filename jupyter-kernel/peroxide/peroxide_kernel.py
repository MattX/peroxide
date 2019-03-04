import inspect

import pexpect
from ipykernel.kernelbase import Kernel


class PeroxidePexpectKernel(Kernel):
    implementation = 'IPython'
    implementation_version = '7.3.0'
    language = 'Peroxide Scheme'
    language_version = '0.1.0'
    language_info = {
        'name': 'Scheme',
        'mimetype': 'text/x-scheme',
        'file_extension': '.scm',
    }
    banner = "Peroxide Scheme Kernel"

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.log.error(inspect.getfile(self.__class__))
        self.child = pexpect.spawnu("/Users/matthieu/src/rustscheme/jupyter-kernel/env/share/jupyter/kernels/peroxide/peroxide --no-readline")
        self.child.setecho(False)
        self.child.expect(">>> ")

    def do_execute(self, code, silent, store_history=True, user_expressions=None, allow_stdin=False):
        ret = []
        idx = 0

        for line in code.splitlines():
            self.child.sendline(line)
            idx = self.child.expect([">>> ", r"\.\.\. "])
            if idx == 0:
                ret.append(self.child.before)

        if idx == 1:
            self.child.sendline("#\\invalid")  # invalid token
            self.child.expect("Error: .*")
            ret.append("** Incomplete expression.")

        if not silent:
            stream_content = {'name': 'stdout', 'text': ''.join(ret)}
            self.send_response(self.iopub_socket, 'stream', stream_content)

        return {'status': 'ok',
                # The base class increments the execution count
                'execution_count': self.execution_count,
                'payload': [],
                'user_expressions': {},
                }
