use crate::solver::{DWaveAnnealerInner, ProblemType};
use crate::ApiError;
use annealers::node::SingleNode;
use annealers::repr::BinaryRepr;
use annealers::solution::SingleSolution;
use serde_json::Value;
use std::collections::HashMap;

const TRUE_VAL: bool = true;
const FALSE_VAL: bool = false;
const STRING_NONE_VAL: Option<String> = None;

#[derive(Deserialize)]
pub struct SolverAnswer {
	#[serde(rename = "type")]
	pub problem_type: ProblemType,
	pub answer: SolverAnswerInner,
}

#[serde(tag = "format")]
#[derive(Deserialize)]
enum SolverAnswerInner {
	#[serde(rename = "qp")]
	Qp {
		active_variables: String,
		#[serde(default)]
		num_occurrences: Option<String>,
		energies: String,
		solutions: String,
		num_variables: usize,
	},
	#[serde(rename = "bq")]
	Bq {
		data: BqSamplesetData, // sampleset
	},
}

#[derive(Deserialize)]
struct BqSamplesetData {
	// "SPIN" or "ISING"
	variable_type: String,
	num_variables: usize,
	// length == num_variables
	variable_labels: Vec<Value>,

	// {'timing': {'qpu_sampling_time': 315,
	//  'qpu_anneal_time_per_sample': 20,
	//  'qpu_readout_time_per_sample': 274,
	info: Value,
	// Additional information per sample.
	// The length of Vec == num_samples
	// Value is encoded ndarray
	vectors: BqSamplesetVectors,

	// encoded ndarray of (num_samples, num_variables)  0 or 1 (regardness of variable_type)
	sample_data: NdArray,
	#[serde(default)]
	sample_packed: bool,
}

#[derive(Deserialize)]
struct BqSamplesetVectors {
	// ndarray (num_samples)
	energy: NdArray,
	// ndarray (num_samples)
	num_occurrences: Option<NdArray>,
	#[serde(flatten)]
	_other: HashMap<String, Vec<Value>>,
}

// We only support 1-dim or 2-dim
#[derive(Deserialize)]
struct NdArray {
	#[serde(default)]
	use_bytes: bool,
	data: Value,
	data_type: String, // dtype == 'int32' and so on
	shape: Vec<usize>,
}

impl NdArray {
	fn get_buf(data: &Value) -> Result<Vec<u8>, ApiError> {
		if let Some(s) = data.as_str() {
			Ok(s.to_owned().into_bytes())
		} else if let Some(a) = data.as_array() {
			let mut ret = Vec::with_capacity(a.len());
			for v in a.iter() {
				if let Some(i) = v.as_i64() {
					ret.push(i as u8)
				} else {
					return Err(ApiError::Api("bad value".to_owned()));
				}
			}
			Ok(ret)
		} else {
			Err(ApiError::Api("bad value".to_owned()))
		}
	}

	fn to_1d_f64_arr(&self) -> Result<Vec<f64>, ApiError> {
		let size = match self.data_type.as_str() {
			"float32" => std::mem::size_of::<f32>(),
			"float64" => std::mem::size_of::<f64>(),
			_ => return Err(ApiError::Api("bad type".to_owned())),
		};
		let size = std::mem::size_of::<f64>();
		if let &[items] = self.shape.as_slice() {
			let mut ret = Vec::new();
			if self.use_bytes {
				let v = Self::get_buf(&self.data)?;
				if v.len() != items * size {
					return Err(ApiError::Api("bad data".to_owned()));
				}
				let mut iter = v.into_iter();
				for i in 0..items {
					let a: Vec<u8> = iter.take(size).collect();
					ret.push(match size {
						32 => f32::from_le_bytes([a[0], a[1], a[2], a[3]]) as f64,
						64 => f64::from_le_bytes([a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]]),
						_ => panic!(),
					});
				}
			} else {
				if let Some(v) = self.data.as_array() {
					for item in v.iter() {
						if let Some(f) = item.as_f64() {
							ret.push(f);
						} else {
							return Err(ApiError::Api("Bad val".to_owned()));
						}
					}
				} else {
					return Err(ApiError::Api("Bad val".to_owned()));
				}
			}
			if ret.len() != items {
				return Err(ApiError::Api("shape mismatch".to_owned()));
			}
			Ok(ret)
		} else {
			Err(ApiError::Api("shape mismatch".to_owned()))
		}
	}

	fn to_1d_i32_arr(&self) -> Result<Vec<i32>, ApiError> {
		let size = match self.data_type.as_str() {
			"int8" => std::mem::size_of::<i8>(),
			"int16" => std::mem::size_of::<i16>(),
			"int32" => std::mem::size_of::<i32>(),
			"int64" => std::mem::size_of::<i64>(),
			_ => return Err(ApiError::Api("bad type".to_owned())),
		};
		let size = std::mem::size_of::<f64>();
		if let &[items] = self.shape.as_slice() {
			let mut ret = Vec::new();
			if self.use_bytes {
				let v = Self::get_buf(&self.data)?;
				if v.len() != items * size {
					return Err(ApiError::Api("bad data".to_owned()));
				}
				let mut iter = v.into_iter();
				for i in 0..items {
					let a: Vec<u8> = iter.take(size).collect();
					ret.push(match size {
						8 => i8::from_le_bytes([a[0]]) as i32,
						16 => i16::from_le_bytes([a[0], a[1]]) as i32,
						32 => i32::from_le_bytes([a[0], a[1], a[2], a[3]]),
						64 => i64::from_le_bytes([a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]])
							as i32,
						_ => panic!(),
					});
				}
			} else {
				if let Some(v) = self.data.as_array() {
					for item in v.iter() {
						if let Some(f) = item.as_f64() {
							ret.push(f);
						} else {
							return Err(ApiError::Api("Bad val".to_owned()));
						}
					}
				} else {
					return Err(ApiError::Api("Bad val".to_owned()));
				}
			}
			if ret.len() != items {
				return Err(ApiError::Api("shape mismatch".to_owned()));
			}
			Ok(ret)
		} else {
			Err(ApiError::Api("shape mismatch".to_owned()))
		}
	}

	fn to_2d_i32_arr(&self) -> Result<Vec<Vec<i32>>, ApiError> {
		let size = match self.data_type.as_str() {
			"int32" => std::mem::size_of::<i32>(),
			_ => return Err(ApiError::Api("bad type".to_owned())),
		};
		let size = std::mem::size_of::<f64>();
		if let &[rows, cols] = self.shape.as_slice() {
			let mut ret = Vec::with_capacity(rows);
			if self.use_bytes {
				let v = Self::get_buf(&self.data)?;
				if v.len() != rows * cols * size {
					return Err(ApiError::Api("bad data".to_owned()));
				}
				let mut iter = v.into_iter();
				for i in 0..rows {
					let mut inner = Vec::with_capacity(cols);
					for j in 0..cols {
						let a: Vec<u8> = iter.take(size).collect();
						inner.push(match size {
							32 => i32::from_le_bytes([a[0], a[1], a[2], a[3]]),
							_ => panic!(),
						});
					}
					ret.push(inner);
				}
			} else {
				if let Some(v) = self.data.as_array() {
					for item in v.iter() {
						let mut inner = Vec::with_capacity(cols);
						if let Some(v2) = item.as_array() {
							for item in v.iter() {
								if let Some(f) = item.as_f64() {
									ret.push(f);
								} else {
									return Err(ApiError::Api("Bad val".to_owned()));
								}
							}
							if inner.len() != cols {
								return Err(ApiError::Api("shape mismatch".to_owned()));
							}
						} else {
							return Err(ApiError::Api("bad val".to_owned()));
						}
						ret.push(inner);
					}
				} else {
					return Err(ApiError::Api("Bad val".to_owned()));
				}
			}
			if ret.len() != rows {
				return Err(ApiError::Api("shape mismatch".to_owned()));
			}
			Ok(ret)
		} else {
			Err(ApiError::Api("shape mismatch".to_owned()))
		}
	}

	fn unpack_2d_arr(&self, variables: usize) -> Result<Vec<Vec<bool>>, ApiError> {
		let arr = self.to_2d_i32_arr()?;
		let mut ret = Vec::new();
		for row in arr.into_iter() {
			let inner = row
				.into_iter()
				.flat_map(|col| col.to_le_bytes().into_iter())
				.flat_map(|mut byte| {
					let mut v = Vec::with_capacity(8);
					for _ in 0..8 {
						v.push((byte & 1) != 0);
						*byte >>= 1;
					}
					v.into_iter()
				})
				.take(variables)
				.collect::<Vec<_>>();
			if inner.len != variables {
				return Err(ApiError::Api("Bad encoding".to_owned()));
			}
			ret.push(inner);
		}
		Ok(ret)
	}
}

impl SolverAnswerInner {
	fn build_i32_arr(input: &[u8]) -> Option<Vec<i32>> {
		let size = std::mem::size_of::<i32>();
		let mut ret = Vec::with_capacity((input.len() - 1) / size + 1);
		let mut iter = input.iter();
		loop {
			let v = iter.take(size).collect::<Vec<_>>();
			if v.len() == 0 {
				return Some(ret);
			} else if v.len() != size {
				return None;
			}
			ret.push(i32::from_le_bytes(v));
		}
	}

	fn build_f64_arr(input: &[u8]) -> Option<Vec<f64>> {
		let size = std::mem::size_of::<f64>();
		let mut ret = Vec::with_capacity((input.len() - 1) / size + 1);
		let mut iter = input.iter();
		loop {
			let v = iter.take(size).collect::<Vec<_>>();
			if v.len() == 0 {
				return Some(ret);
			} else if v.len() != size {
				return None;
			}
			ret.push(f64::from_le_bytes([
				*v[0], *v[1], *v[2], *v[3], *v[4], *v[5], *v[6], *v[7],
			]));
		}
	}

	pub fn decode<M: SingleNode>(
		self,
		inner: &DWaveAnnealerInner,
	) -> Result<Vec<SingleSolution<M>>, ApiError> {
		match self {
			Self::Qp {
				active_variables,
				num_occurrences,
				energies,
				solutions,
				num_variables,
			} => {
				if let DWaveAnnealerInner::Structured {
					qubits,
					couplers: _,
				} = inner
				{
					Self::decode_qp(
						&active_variables,
						num_occurrences.as_str(),
						&energies,
						&solutions,
						num_variables,
						qubits,
					)
				} else {
					Err(ApiError::Api("Bad answer".to_owned()))
				}
			}
			Self::Bq { data } => match inner {
				DWaveAnnealerInner::Bqm => Self::decode_bq(&data),
				DWaveAnnealerInner::Dqm => unimplemented!(),
				_ => Err(ApiError::Api("Bad answer".to_owned())),
			},
		}
	}

	fn decode_bq<M: SingleNode<RealType = f64>>(
		data: &BqSamplesetData,
	) -> Result<Vec<SingleSolution<M>>, ApiError> {
		let mut sample = if data.sample_packed {
			data.sample_data.unpack_2d_arr(data.num_variables)?
		} else {
			data.sample_data
				.to_2d_i32_arr()?
				.into_iter()
				.map(|arr| arr.into_iter().map(|val| val == 1).collect())
				.collect()
		};
		let energies = data.vectors.energy.to_1d_f64_arr();
		let num_occurrences = data.vectors.num_occurrences.map(|o| o.to_1d_i32_arr());
		let mut ret = Vec::with_capacity(sample.len());
		for (i, (sample, energy)) in sample.into_iter().zip(energies.into_iter()).enumerate() {
			let mut item = SingleSolution::from_vec(sample.as_slice());
			item.energy = Some(energy);
			if let Some(occur) = num_occurrences.map(|no| no[i]) {
				item.occurrences = occur;
			}
			ret.push(item);
		}
		Ok(ret)
	}

	fn decode_qp<M: SingleNode>(
		active_variables: &str,
		num_occurrences: Option<&str>,
		energies: &str,
		solutions: &str,
		num_variables: usize,
		qubits: &[i32],
	) -> Result<Vec<SingleSolution<M>>, ApiError> {
		let energies: Vec<f64> = SolverAnswerInner::build_f64_arr(&base64::decode(&energies)?)
			.ok_or(ApiError::Api("energies parse error".to_owned()))?;
		let times = energies.len();
		let active_variables: Vec<i32> =
			SolverAnswerInner::build_i32_arr(&base64::decode(&active_variables)?)
				.ok_or(ApiError::Api("active_variables parse error".to_owned()))?;
		let column_length = (active_variables.len() + 7) / 8;
		let active_variables = active_variables
			.into_iter()
			.enumerate()
			.map(|(i, v)| (v, i))
			.collect::<HashMap<i32, usize>>();
		let num_occurrences = if let Some(num_occurrences) = num_occurrences {
			SolverAnswerInner::build_i32_arr(&base64::decode(&active_variables)?)
				.ok_or(ApiError::Api("num_occurrences parse error".to_owned()))?
		} else {
			Some(1).into_iter().cycle().take(energies.len()).collect()
		};
		if energies.len() != num_occurrences.len() {
			return Err(ApiError::Api("num_occurrences error".to_owned()));
		}
		let solutions = base64::decode(solutions)?;
		if solutions.len() != column_length * energies.len() {
			return Err(ApiError::Api("Solution Error".to_owned()));
		}
		let mut ret = Vec::new();
		for (i, (energy, occur)) in energies
			.into_iter()
			.zip(num_occurrences.into_iter())
			.enumerate()
		{
			let index = column_length * i;
			let mut state = unsafe { BinaryRepr::with_len_unchecked(qubits.len()) };
			for (j, qubit) in qubits.iter().enumerate() {
				if let Some(k) = active_variables.get(qubit) {
					let val = (solutions[i + (k / 8)] & (1 << (7 - (k % 8)))) != 0;
					state.set(j, val);
				} else {
					return Err(ApiError::Api("Broken qubit".to_owned()));
				}
			}
			let mut item = SingleSolution::from_state(state);
			item.energy = Some(energy);
			item.occurrences = occur;
			ret.push(item);
		}
		Ok(ret)
	}
}
