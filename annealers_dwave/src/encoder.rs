use crate::solver::ProblemData;
use crate::{ApiError, Result};
use std::collections::{HashMap, HashSet};

const BQM_MAGIC_PREFIX: &[u8] = b"DIMODBQM";
const BQM_VERSION: [u8; 2] = [1, 0];

pub fn encode_bqm(
	offset: f64,
	qubits: &[f64],
	couplers: &HashMap<(usize, usize), f64>,
	is_spin: bool,
) -> Vec<u8> {
	let mut neighbor_view: Vec<usize> = Vec::with_capacity(qubits.len());
	let mut converted_couplers: Vec<(i32, f64)> = Vec::with_capacity(couplers.len() * 2);
	for (i, j) in couplers.keys() {
		assert!(i < &qubits.len());
		assert!(j < &qubits.len());
	}
	for i in 0..qubits.len() {
		neighbor_view.push(converted_couplers.len());
		for j in 0..qubits.len() {
			match (couplers.get(&(i, j)), couplers.get(&(i, j))) {
				(Some(w), Some(w2)) => {
					assert_eq!(w, w2);
					converted_couplers.push((j as i32, *w));
				}
				(None, Some(w)) | (Some(w), None) => {
					converted_couplers.push((j as i32, *w));
				}
				_ => (),
			}
		}
	}
	// header
	let mut ret = generate_header(qubits.len(), couplers.len(), is_spin);
	// offset
	ret.extend_from_slice(&offset.to_le_bytes());
	// linear
	for (nv, q) in neighbor_view.iter().chain(qubits.iter()) {
		ret.extend_from_slice(nv.to_le_bytes());
		ret.extend_from_slice(q.to_le_bytes());
	}
	// quad
	for (i, w) in converted_couplers.iter() {
		ret.extend_from_slice(&i.to_le_bytes());
		ret.extend_from_slice(&w.to_le_bytes());
	}
	// variables -> empty (when VERSION == 1.0 or index_labelled)
	ret
}

pub fn encode_qp(
	qubits: &[usize],
	couplers: &[(usize, usize)],
	h: &HashMap<usize, f64>,
	neighbors: &HashMap<(usize, usize), f64>,
) -> Result<ProblemData> {
	let qubits_set = qubits.iter().cloned().collect::<HashSet<_>>();
	let couplers_set = qubits.iter().cloned().collect::<HashSet<_>>();
	let mut active_qubits = HashSet::new();
	for q in h.keys() {
		if qubits_set.get(q).is_none() {
			return Err(ApiError::Problem(format!("Qubit {} in `h` not exists", q)));
		}
		active_qubits.insert(q.clone());
	}
	for (i, j) in neighbors.keys() {
		if couplers_set.get(&(*i, *j)).is_none() && couplers_set.get(&(*j, *i)).is_none() {
			return Err(ApiError::Problem(format!(
				"Coupler ({}, {}) in `neighbors` not exists",
				i, j
			)));
		}
		active_qubits.insert(i.clone());
		active_qubits.insert(j.clone());
	}
	// set NAN if the qubits is used in `h` nor `couplers`
	let lin = qubits
		.iter()
		.map(|q| {
			h.get(q)
				.unwrap_or(active_qubits.get(q).map(|_| 0.0).unwrap_or(f64::NAN))
		})
		.flat_map(|f| *f.to_le_bytes().into_iter())
		.collect::<Vec<_>>();
	let quad = couplers
		.iter()
		.filter(|(i, j)| active_qubits.get(i).is_none() || active_qubits.get(j).is_none())
		.map(|(i, j)| *neighbors.get(&(*i, *j)));
	Ok(ProblemData::Qp {
		lin: base64::encode(&lin),
		quad: base64::encode(&quad),
		offset: 0.0,
	})
}

pub fn generate_header(num_qubits: usize, num_couplers: usize, is_spin: bool) -> Vec<u8> {
	let variables = if BQM_VERSION[0] == 1 {
		&format!("{:?}", (0..num_qubits).collect::<Vec<_>>())
	} else {
		"false"
	};
	let header_data = format!(
		r#"{{"dtype": "float64", "itype": "uint32", "ntype": "uint64", "shape": [{}, {}], "type": "BinaryQuadraticModel", "variables": {}, "vartype": "{}"}}{}"#,
		num_qubits,
		num_couplers,
		variables,
		if is_spin { "SPIN" } else { "BINARY" },
		"\n"
	);
	if BQM_VERSION[0] == 1 {
		// add variables in header
	}
	let ret = Vec::new();
	ret.extend_from_slice(BQM_MAGIC_PREFIX);
	ret.extend_from_slice(&BQM_VERSION);
	let header_len: u32 = ((ret.len() as u32 + header_data.len() as u32 + 4 - 1) / 16 + 1) * 16;
	ret.extend_from_slice(&header_len.to_le_bytes());
	ret.extend_from_slice(header_data.as_bytes());
	while ret.len() < (header_len as usize) {
		ret.push(' ' as u8);
	}
	assert_eq!(header_len % 16, 0);
	ret
}
