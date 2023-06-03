extern crate async_trait;
use crate::session::DWaveSession;
use crate::ApiError;
use annealers::model::SingleModel;
use annealers::node::SingleNode;
use annealers::set::NodeSet;
use annealers::solution::SingleSolution;
use annealers::solver::{AsyncSolver, Solver, SolverGenerator, StructuredSolverGenerator};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum SolverCategory {
	#[serde(rename = "software")]
	Software,
	#[serde(rename = "qpu")]
	Qpu,
	/// quantum-classical hybrid; typically one or more classical algorithms run
	/// on the problem while outsourcing to a quantum processing unit (QPU)
	/// parts of the problem where it benefits most.
	#[serde(rename = "hybrid")]
	Hybrid,
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum ProblemType {
	/// (for qpu-like solvers) Ising model problems; use −1/1 -valued
	#[serde(rename = "ising")]
	Ising,
	/// (for qpu-like solvers) Quadratic unconstrained binary optimization
	///   (QUBO) problems; use 0/1- valued variables.
	#[serde(rename = "qubo")]
	Qubo,
	/// (for hybrid solvers) binary quadratic model (BQM) problems; use
	///   0/1 -valued variables and −1/1- valued variables.
	#[serde(rename = "bqm")]
	Bqm,
	/// (for hybrid solvers) discrete quadratic model (DQM) problems;
	///   use variables that can represent a set of values such as
	///   {red,green,blue,yellow} or {3.2,67}.
	#[serde(rename = "dqm")]
	Dqm,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum TopologyType {
	#[serde(rename = "chimera")]
	Chimera,
	Pegasus,
	#[serde(other)]
	Other,
}

#[allow(unused)]
#[derive(Deserialize)]
pub struct SolverTopology {
	#[serde(rename = "type")]
	topology: TopologyType,

	/// Shape of the QPU graph
	shape: (usize, usize, usize),
}

#[allow(unused)]
#[derive(Deserialize, Clone)]
pub struct SolverProperties {
	/// Indicates what problem types are supported for the solver.
	pub supported_problem_types: BTreeSet<ProblemType>,

	/// List of the parameters supported for the solver and descriptions
	pub parameters: HashMap<String, Option<String>>,

	/// Type of solver
	pub category: SolverCategory,

	/// Rate at which user or project quota is consumed for the solver. Time is
	/// deducted from your quota according to:
	///     num_seconds / quota_conversion_rate
	/// If your quota_conversion_rate is 1, for example, then the rate of quota
	/// consumption is straightforward: 1 second used on a solver deducts 1
	/// second from your quota. Different solver types may consume quota at
	/// different rates.
	pub quota_conversion_rate: usize,

	/// Include other properties
	#[serde(flatten)]
	pub properties: HashMap<String, Value>,
}

#[derive(Deserialize, Clone)]
pub struct SolverInfo {
	pub id: String,
	pub status: String,
	pub description: String,
	pub properties: SolverProperties,
	pub avg_load: f64,
}

#[serde(rename_all = "lowercase")]
#[derive(Serialize)]
#[serde(tag = "format")]
pub(crate) enum ProblemData {
	Ref {
		data: String,
	},
	Qp {
		lin: String,
		quad: String,
		offset: f64,
	},
}

#[derive(Clone)]
pub struct DWaveAnnealerGenerator {
	pub info: SolverInfo, // pub for reference (avg_load, description, etc)
	session: Arc<DWaveSession>,
	inner: DWaveAnnealerInner,
	pub params: HashMap<String, Value>,
}

impl DWaveAnnealerGenerator {
	pub(crate) fn from_info(
		info: SolverInfo,
		session: Arc<DWaveSession>,
	) -> Result<Self, ApiError> {
		let inner = DWaveAnnealerInner::from_info(&info)?;
		Ok(Self {
			info,
			session,
			inner,
			params: HashMap::new(),
		})
	}

	pub fn get_property<T: AsRef<str>>(&self, key: T) -> Option<&Value> {
		self.info.properties.properties.get(&key)
	}
}

impl<P: SingleModel> SolverGenerator<P> for DWaveAnnealerGenerator {
	type SolverType = DWaveAnnealer<P::NodeType>;
	type ErrorType = ApiError;

	fn generate(&self, model: &P) -> Result<Self::SolverType, Self::ErrorType> {
		let (problem, problem_type) = match &self.inner {
			DWaveAnnealerInner::Structured { qubits, couplers } => {
				unimplemented!()
			}
			DWaveAnnealerInner::Bqm => {
				let mut h = std::iter::repeat(0.0)
					.take(model.size())
					.collect::<Vec<_>>();
				let mut couplers = HashMap::new();
				for set in model.prods().into_iter() {
					let w = model.get_weight(&set);
					if let [i, j] = set.into_vec() {
						couplers.insert((i, j), w);
					} else {
						panic!();
					}
				}
				let encoded = crate::encoder::encode_bqm(0.0, &h, &couplers, true)?;
				let problem_id = self.session.upload_problem(&encoded, None)?;
				(ProblemData::Ref { data: problem_id }, ProblemType::Bqm)
			}
			DWaveAnnealerInner::Dqm => return Err(ApiError::NotImplemented),
		};
		Ok(DWaveAnnealer {
			id: self.info.id,
			problem,
			problem_type,
			params: self.params.clone(),
			session: self.session.clone(),
			_phantom: PhantomData,
		})
	}
}

impl<P: SingleModel> StructuredSolverGenerator<P> for DWaveAnnealerGenerator {
	fn nodes(&self) -> Box<dyn Iterator<Item = usize>> {
		match &self.inner {
			DWaveAnnealerInner::Structured {
				qubits,
				couplers: _,
			} => Box::new(qubits.iter().map(|i| *i as usize)) as Box<dyn Iterator<Item = usize>>,
			_ => unimplemented!(),
		}
	}

	fn prods(&self) -> Box<dyn Iterator<Item = BTreeSet<usize>>> {
		match &self.inner {
			DWaveAnnealerInner::Structured {
				qubits: _,
				couplers,
			} => Box::new(couplers.iter().map(|(i, j)| {
				let mut set = BTreeSet::new();
				set.insert(*i as usize);
				set.insert(*j as usize);
				set
			})) as Box<dyn Iterator<Item = usize>>,
			_ => unimplemented!(),
		}
	}
}

pub struct DWaveAnnealer<M: SingleNode> {
	id: String,
	inner: DWaveAnnealerInner,
	problem: ProblemData,
	problem_type: ProblemType,
	params: HashMap<String, Value>,
	session: Arc<DWaveSession>,
	_phantom: PhantomData<M>,
}

#[derive(Clone)]
pub(crate) enum DWaveAnnealerInner {
	Structured {
		// encoding formats = qp
		qubits: Vec<i32>,
		couplers: BTreeSet<(i32, i32)>,
	},
	Bqm, // encoding format = bq
	Dqm, // encoding formats = bq
}

impl DWaveAnnealerInner {
	fn from_info(info: &SolverInfo) -> Result<Self, ApiError> {
		let pt = &info.properties.supported_problem_types;
		if pt.get("ising").is_some() && pt.get("qubo").is_some() {
			if let Some(qubits) = info.properties.properties.get("qubits") {
				if let Some(couplers) = info.properties.properties.get("couplers") {
					return Ok(Self::Structured {
						qubits: qubits
							.as_array()
							.ok_or(ApiError::Api("Bad qubits".to_owned()))?
							.iter()
							.map(|o| {
								o.as_u64().ok_or(ApiError::Api("Bad qubit".to_owned()))? as usize
							})
							.collect(),
						couplers: couplers
							.as_array()
							.ok_or(ApiError::Api("Bad couplers".to_owned()))?
							.iter()
							.map(
								|o| {
									if let &[q1, q2] = o
										.as_array()
										.ok_or(ApiError::Api("Bad coupler".to_owned()))?
										.as_ref()
									{
										Ok((q1, q2))
									} else {
										Err(ApiError::Api("Bad coupler".to_owned()))
									}?
								},
							)
							.collect(),
					});
				}
			}
		}
		if pt.get("bqm").is_some() && pt.get("dqm").is_some() {
			unimplemented!()
		}
		Err(ApiError::Api("Bad solver".to_owned()))
	}
}

impl<M: SingleNode<RealType = f64>> Solver for DWaveAnnealer<M> {
	type ErrorType = ApiError;
	type SolutionType = SingleSolution<M>;
}

#[async_trait]
impl<M: SingleNode<RealType = f64>> AsyncSolver for DWaveAnnealer<M> {
	/// The high-layer interface to start annealing process and return the
	/// result.
	async fn solve_async(&self) -> Result<Vec<SingleSolution<M>>, ApiError> {
		let ans = self
			.session
			.submit_problem(&self.id, &self.problem, self.problem_type, &self.params)
			.await?;
		ans.answer.decode(&self.inner)
	}
}
