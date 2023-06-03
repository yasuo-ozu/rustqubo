use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SolverPropertiesInner {
	Hybrid {
		/// Minimum required run time, in seconds, the solver must be allowed to
		/// work on the given problem. Specifies the minimum time required for
		/// the given problem, as a piecewiselinear curve defined by a set of
		/// floating-point pairs. The second element is the minimum
		/// required time; the first element in each pair is some measure of the
		/// problem, dependent on the solver:
		/// * For hybrid BQM solvers, this is the number of variables.
		/// * For hybrid DQM solvers, this is a combination of the numbers of
		///   interactions, variables, and cases that reflects the “density” of
		///   connectivity between the problem’s
		/// variables.
		/// The minimum time for any particular problem is a linear
		/// interpolation calculated on two pairs that represent the relevant
		/// range for the given measure of the problem. For example, if
		/// minimum_time_limit for a hybrid BQM solver were [[1,0.1],[100,10.0],
		/// [1000,20.0]], then the minimum time for a 50-variable problem
		/// would be 5 seconds, the linear interpolation of the first two pairs
		/// that represent problems with between 1 to 100 variables.
		minimum_time_limit: Vec<(usize, f64)>,

		/// Maximum allowed run time, in hours, that can be specified for the
		/// solver.
		maximum_time_limit_hrs: usize, // 24

		/// Maximum number of problem variables accepted by the solver.
		maximum_number_of_variables: usize, // 5000, 10000, 1000000

		/// Maximum number of biases, both linear and quadratic in total,
		/// accepted by the solver
		maximum_number_of_biases: usize,

		/// Version number of the solver (e.g., "1.0").
		version: String,
	},

	/// Representation of Qpu or Software annealer.
	QpuLike {
		/// Indices of the working qubits in the working graph. For example,
		/// [0,1,2,3,...]
		qubits: Vec<usize>,

		/// Couplers in the working graph. A coupler contains two elements [q1,
		/// q2], where both q1 and q2 appear in the list of working qubits, in
		/// the range [0, num_qubits - 1] and in ascending order (i.e., q1 <
		/// q2). These are the couplers that can be programmed with nonzero J
		/// values; for example, [[0,4],[1,4],[2,4],...]
		couplers: Vec<(usize, usize)>,

		/// Total number of qubits, both working and nonworking, in the QPU; for
		/// example, 2048.
		num_qubits: usize,

		/// Range of values possible for the number of reads that you can
		/// request for a problem; for example, [1,1000].
		num_reads_range: (usize, usize),

		/// Indicates the topology type (chimera or pegasus) and shape of the
		/// QPU graph. For example, the following topology is a C16 Chimera
		/// graph, meaning that the QPU has 16 x 16 blocks of Chimera unit
		/// cells, and each unit cell has K4,4 connectivity
		/// `{"type": "chimera", "shape": [16, 16, 4]}`
		topology: SolverTopology,

		/// May hold attributes about a solver that you can use to have a client
		/// program choose one solver over another.
		/// For example, the following attribute identifies a solver as
		/// `lower-noise: "tags": ["lower_noise"]`
		tags: Vec<String>,

		/// Only for sampling emulators (Software)
		beta_range: Option<(f64, f64)>,

		/// Only for sampling emulators (Software)
		default_beta: usize,

		/// Properties for Qpu (not Software) solvers
		#[serde(flatten)]
		hardware: Option<QpuProperties>,
	},
}

#[allow(unused)]
#[derive(Deserialize)]
pub struct QpuProperties {
	/// Array of ranges of valid anneal offset values, in normalized offset
	/// units, for each qubit. The negative values represent the largest number
	/// of normalized offset units by which a qubit’s anneal path may be
	/// delayed. The positive values represent the largest number of normalized
	/// offset units by which a qubit’s anneal path may be advanced.
	anneal_offset_ranges: Vec<(f64, f64)>,
	/// Quantization step size of anneal offset values in normalized units.
	anneal_offset_step: f64,

	/// Quantization step size in physical units (annealing flux-bias units):
	/// Φ0.
	anneal_offset_step_phi0: f64,

	/// Range of time, in microseconds, possible for one anneal (read). The
	/// lower limit in this range is the fastest quench possible for this
	/// solver. When adjusting the anneal schedule, using either the
	/// annealing_time or anneal_schedule parameter, ensure that you do not
	/// exceed the limits in this range.
	annealing_time_range: (usize, usize),

	/// name of the solver
	chip_id: String,

	/// Default time, in microseconds, for one anneal (read). You can change the
	/// annealing time for a given problem by using the annealing_time or
	/// anneal_schedule parameters, but do not exceed the upper limit given by
	/// the annealing_time_range property
	default_annealing_time: usize,

	/// Default time, in microseconds, that the system waits after programming
	/// the QPU for it to return to base temperature. This value contributes to
	/// the total qpu_programming_time, which is returned by SAPI with the
	/// problem solutions. You can change this value using the
	/// programming_thermalization parameter, but be aware that values lower
	/// than the default accelerate solving at the expense of solution quality
	default_programming_thermalization: usize,

	/// Default time, in microseconds, that the system waits after each state is
	/// read from the QPU for it to cool back to base temperature. This value
	/// contributes to the qpu_delay_time_per_sample field, which is returned by
	/// SAPI with the problem solutions.
	default_readout_thermalization: usize,

	/// Extended range of values possible for the coupling strengths (quadratic
	/// coefficients), J, for this solver. Strong negative couplings may be
	/// necessary for some embeddings; however, such chains may require
	/// additional calibration through the flux_biases parameter to compensate
	/// for biases introduced by strong negative couplings.
	extended_j_range: (i32, i32),

	/// Range of the time-dependent gain applied to qubit biases for this
	/// solver. When setting this gain, using the h_gain_schedule parameter,
	/// ensure that you do not exceed the limits in this range.
	h_gain_schedule_range: (i32, i32),

	/// Range of values possible for the qubit biases (linear coefficients), h,
	/// for this solver. The auto_scale parameter, which rescales h and J values
	/// in the problem to use as much of the range of h (h_range) and the range
	/// of J (j_range) as possible, enables you to submit problems with values
	/// outside these ranges and have the system automatically scale them
	/// to fit.
	h_range: (i32, i32),

	/// Range of values possible for the coupling strengths (quadratic
	/// coefficients), J, for this solver.
	/// The auto_scale parameter, which rescales h and J values in the problem
	/// to use as much of the range of h (h_range) and the range of J (j_range)
	/// as possible, enables you to submit problems with values outside these
	/// ranges and have the system automatically scale them to fit.
	j_range: (i32, i32),

	/// Maximum number of points permitted in a PWL waveform submitted to change
	/// the default anneal schedule. Check this value before defining a new
	/// schedule with the anneal_schedule parameter. For reverse annealing, the
	/// maximum number of points allowed is one more than the number given in
	/// the max_anneal_schedule_points property.
	max_anneal_schedule_points: usize,

	/// Maximum number of points permitted in a PWL waveform submitted to set a
	/// timedependent gain on linear coefficients (qubit biases, see the h
	/// parameter) in the Hamiltonian. Check this value before using the
	/// h_gain_schedule parameter.
	max_h_gain_schedule_points: usize,

	/// Coupling range permitted per qubit for this solver. Check this property
	/// when using an extended J range to strongly couple qubits in a chain.
	/// Strong negative couplings may be necessary for some embeddings; however,
	/// chains may require additional calibration through the flux_biases
	/// parameter to compensate for biases introduced by strong negative
	/// couplings.
	per_qubit_coupling_range: (i32, i32),

	/// Range of time, in microseconds, that a problem can run.
	/// The upper limit of this range is calculated according to the following
	/// formula:
	///     $Duration = ((P1 + P2) ∗ P3) + P4$
	/// where P1, P2, P3, and P4 are the values specified for the
	/// annealing_time, readout_thermalization, num_reads (samples), and
	/// programming_thermalization parameters, respectively
	problem_run_duration_range: (usize, usize),

	/// Range of time, in microseconds, possible for the system to wait after
	/// programming the QPU for it to cool back to base temperature This value
	/// contributes to the total qpu_programming_time, which is returned by SAPI
	/// with the problem solutions. You can change this value using the
	/// programming_thermalization parameter, but be aware that values lower
	/// than the default accelerate solving at the expense of solution quality.
	/// The default value for a solver is given in the
	/// default_programming_thermalization property.
	programming_thermalization_range: (usize, usize),

	/// Range of time, in microseconds, possible for the system to wait after
	/// each state is read from the QPU for it to cool back to base temperature.
	/// This value contributes to the qpu_delay_time_per_sample field, which is
	/// returned by SAPI with the problem solutions.
	readout_thermalization_range: (usize, usize),

	/// Flag indicating whether this solver is a VFYC (virtual full-yield chip)
	/// solver.
	vfyc: bool,
}
