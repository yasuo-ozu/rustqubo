from setuptools import setup  # type: ignore
from setuptools_rust import Binding, RustExtension  # type: ignore

setup_kwargs = {
    'packages': ['rustqubo'],
    "rust_extensions": [
        RustExtension(
            'rustqubo.rustqubo', 'Cargo.toml',
            binding=Binding.PyO3,
            args=["--no-default-features", "--crate-type", "cdylib"],
            features=['python'],
        )
    ],
    "zip_safe": False,
}

setup(**setup_kwargs)
