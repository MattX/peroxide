from setuptools import setup

setup(
    name='peroxide_kernel',
    version='1.1',
    packages=['peroxide'],
    description='Simple kernel for Peroxide Scheme',
    long_description='Simple kernel for Peroxide Scheme',
    author='Matthieu Felix',
    author_email='matthieufelix@gmail.com',
    url='https://github.com/MattX/peroxide/jupyter-kernel',
    install_requires=[
        'jupyter_client', 'IPython', 'ipykernel', 'pexpect>=4.6.0'
    ],
    classifiers=[
        'Intended Audience :: Developers',
        'License :: OSI Approved :: Apache 2',
        'Programming Language :: Python :: 3',
    ],
)
