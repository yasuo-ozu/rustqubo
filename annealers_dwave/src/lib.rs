extern crate base64;
extern crate hex;
extern crate ini;
extern crate md5;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate tokio;
#[macro_use]
extern crate serde_derive;

mod decoder;
mod encoder;
mod profile;
pub mod session;
pub mod solver;

pub type Result<T> = std::result::Result<T, ApiError>;

// use crate::prelude::*;
use ini::Error as IniError;
use reqwest::Error as ConnectionError;

#[derive(Debug)]
pub enum ApiError {
	Auth(String),
	LoadConfig(String),
	IniParse(IniError),
	Connection(ConnectionError),
	Api(String),
	Problem(String),
	Base64Decode(base64::DecodeError),
	NotFound,
	Cancelled,
	NotImplemented,
}

impl From<base64::DecodeError> for ApiError {
	fn from(f: base64::DecodeError) -> Self {
		Self::Base64Decode(f)
	}
}

impl From<IniError> for ApiError {
	fn from(f: IniError) -> Self {
		ApiError::IniParse(f)
	}
}

impl From<ConnectionError> for ApiError {
	fn from(f: ConnectionError) -> Self {
		ApiError::Connection(f)
	}
}

impl std::fmt::Display for ApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}
impl std::error::Error for ApiError {}

// #[derive(Clone)]
// pub struct DWaveAnnealerInfo {
// 	pub endpoint: String,
// 	pub token: String,
// 	pub client: ClientType,
// 	pub machine: String,
// }
//
// impl DWaveAnnealerInfo {
// 	/// Generate with default config and given token.
// 	pub fn with_token(token: String) -> Self {
// 		Self {
// 			endpoint: "https://cloud.dwavesys.com/sapi".to_owned(),
// 			token,
// 			client: ClientType::Sw,
// 			machine: "DW_2000Q_5".to_owned(),
// 		}
// 	}
//
// }

// impl AnnealerInfo for DWaveAnnealerInfo {
// 	type AnnealerType = DWaveAnnealer;
// 	type ErrorType = ApiError;
// 	fn build_with_ising(
// 		&self,
// 		h: Vec<f64>,
// 		neighbors: Vec<Vec<(usize, f64)>>,
// 	) -> Result<Self::AnnealerType, ApiError> {
// 		fn build_inner(
// 			py: Python,
// 			num_reads: usize,
// 			endpoint: &str,
// 			token: &str,
// 			solver: &str,
// 			h_ising: Vec<f64>,
// 			neighbors_ising: Vec<Vec<(usize, f64)>>,
// 		) -> PyResult<(PyObject, PyObject, PyTuple, PyDict)> {
// 			let py_dwave = py.import("dwave.cloud")?;
// 			let py_client_class = py_dwave.get(py, "cloud")?.getattr(py, "Client")?;
// 			let py_client = py_client_class.call(py, (endpoint, token, solver), None)?;
// 			let py_solver =
// 				py_client
// 					.getattr(py, "get_solver")?
// 					.call(py, PyTuple::new(py, &[]), None)?;
// 			// TODO: remove dimod dependency (see dwave/cloud/solver.py)
// 			let py_anneal_method = py_solver.get(py, "sample_qubo")?;
// 			let py_args = {
// 				let py_linear = (&h_ising as &[f64]).to_py_object(py);
// 				let py_quad = {
// 					let py_quad = PyDict::new(py);
// 					for (i, neigh) in neighbors_ising.iter().enumerate() {
// 						for (j, weight) in neigh.iter() {
// 							py_quad.set_item(py, (i, j), *weight)?;
// 						}
// 					}
// 					py_quad
// 				};
// 				(py_linear, py_quad).to_py_object(py)
// 			};
// 			let py_kwargs = {
// 				let py_kwargs = PyDict::new(py);
// 				py_kwargs.set_item(py, "num_reads", num_reads)?;
// 				py_kwargs
// 			};
// 			Ok((py_client, py_anneal_method, py_args, py_kwargs))
// 		}
// 		if let None = &self.token {
// 			return Err(ApiError::Auth("Token must not None".to_owned()));
// 		}
// 		let beta_schedule = self.beta.generate_schedule(&h, &neighbors);
// 		match build_inner(
// 			Python::acquire_gil().python(),
// 			self.num_reads,
// 			&self.endpoint,
// 			&self.token.clone().unwrap(),
// 			&self.machine,
// 			h,
// 			neighbors,
// 		) {
// 			Ok((py_client, py_anneal_method, py_args, py_kwargs)) => Ok(DWaveAnnealer {
// 				beta_schedule,
// 				py_client,
// 				py_anneal_method,
// 				py_args,
// 				py_kwargs,
// 			}),
// 			Err(py_err) => Err(ApiError::Python(py_err)),
// 		}
// 	}
// }
//
// pub struct DWaveAnnealer {
// 	beta_schedule: Vec<f64>,
// 	py_client: PyObject,
// 	py_anneal_method: PyObject,
// 	py_args: PyTuple,
// 	py_kwargs: PyDict,
// }
//
// impl Annealer<ApiError> for DWaveAnnealer {
// 	fn anneal(&self) -> Result<Vec<bool>, ApiError> {
// 		fn anneal_inner(py: Python, this: &DWaveAnnealer) -> PyResult<()> {
// 			let ans = this
// 				.py_anneal_method
// 				.call(py, this.py_args, Some(this.py_kwargs))?;
// 		}
// 		match anneal_inner(Python::acquire_gil().python(), self) {
// 			Ok(()) => unimplemented!(),
// 			Err(py_err) => Err(ApiError::Python(py_err)),
// 		}
// 	}
// }
//
// impl std::ops::Drop for DWaveAnnealer {
// 	fn drop(&mut self) {
// 		let gil = Python::acquire_gil();
// 		let py = gil.python();
// 		if let Ok(py_close) = self.py_client.getattr(py, "close") {
// 			let _ = py_close.call(py, PyTuple::new(py, &[]), None);
// 		}
// 	}
// }
