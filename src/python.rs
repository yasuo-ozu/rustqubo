use pyo3::prelude::*;

#[pymodule]
fn rustqubo(_py: Python<'_>, _m: &PyModule) -> PyResult<()> {
	Ok(())
}
