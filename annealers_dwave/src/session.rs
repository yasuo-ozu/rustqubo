use crate::decoder::SolverAnswer;
use crate::solver::{DWaveAnnealerGenerator, ProblemData, ProblemType, SolverCategory, SolverInfo};
use crate::ApiError;
use core::time::Duration;
use ini::Ini;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Proxy};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

macro_rules! builder_pattern {
	($name: ident) => {
		/// Builder function for property $name.
		///
		/// # Args
		/// * ~$name~ - Set property `$name`.
		pub fn $name<'a, S: Into<Cow<'a, str>>>(mut self, $name: S) -> Self {
			self.$name = Some($name.into().into_owned());
			self
		}
	};
	($name: ident, $typ: ty) => {
		/// Builder function for property $name.
		///
		/// # Args
		/// * ~$name~ - Set property `$name`.
		pub fn $name<'a, T: Into<$typ>>(mut self, $name: T) -> Self {
			self.$name = Some($name.into());
			self
		}
	};
}

#[derive(Clone)]
pub struct DWaveApi {
	inner: Arc<DWaveSession>,
	pub category: Option<SolverCategory>,
	pub machine: Option<String>,
}

impl DWaveApi {
	pub fn from_session(session: DWaveSession) -> Self {
		Self {
			inner: Arc::new(session),
			category: None,
			machine: None,
		}
	}

	pub fn from_properties(
		endpoint: Option<String>,
		token: Option<String>,
		proxy: Option<String>,
	) -> Self {
		Self {
			inner: Arc::new(DWaveSession::from_properties(endpoint, token, proxy)),
			category: None,
			machine: None,
		}
	}

	builder_pattern!(category, SolverCategory);
	builder_pattern!(machine);

	/// Load the settings from D-Wave's config file.
	///
	/// Args:
	/// * `fname` - Load settings from given file. If None, use D-Wave's default
	///   config file.
	/// * `profile` - Use given sections of each ini files. If None, use
	///   'profile' config in each ini file.
	pub fn from_file(fname: Option<PathBuf>, profile: Option<&str>) -> Result<Self, ApiError> {
		let mut endpoint = None;
		let mut token = None;
		let mut proxy = None;
		let mut category = None;
		let mut machine = None;
		let files = if let Some(fname) = fname {
			vec![fname]
		} else {
			crate::profile::get_dwave_path()
		};
		let files = files
			.into_iter()
			.filter(|p| Path::exists(p.as_path()))
			.collect::<Vec<_>>();
		if files.len() == 0 {
			return Err(ApiError::LoadConfig("Cannot find file".to_owned()));
		}
		for fname in files.into_iter() {
			let config = Ini::load_from_file(fname.clone())?;
			if let Some(first_section) = config.sections().next() {
				// the ini file is not empty
				let section = profile
					.map(|o| Some(o))
					.or_else(|| config.get_from(first_section, "profile").map(|o| Some(o)))
					.unwrap_or(first_section);
				if let Some(client) = config.get_from(section, "client") {
					// "sw"
					category = Some(match client {
						"sw" => SolverCategory::Software,
						"qpu" => SolverCategory::Qpu,
						"hybrid" => SolverCategory::Hybrid,
						o => {
							return Err(ApiError::LoadConfig(format!(
								"Cannot understarnd client type {} in {}",
								&o,
								fname.to_str().unwrap()
							)))
						}
					});
				}
				if let Some(r_endpoint) = config.get_from(section, "endpoint") {
					endpoint = Some(r_endpoint.to_owned());
				}
				if let Some(solver) = config.get_from(section, "solver") {
					machine = Some(solver.to_owned());
				}
				if let Some(r_token) = config.get_from(section, "token") {
					token = Some(r_token.to_owned());
				}
			};
		}
		let mut ret = Self::from_properties(endpoint, token, proxy);
		ret.category = category;
		ret.machine = machine;
		Ok(ret)
	}

	pub fn get_solver_sync(&self) -> Result<DWaveAnnealerGenerator, ApiError> {
		let mut runtime = tokio::runtime::Runtime::new().unwrap();
		runtime.block_on(self.get_solver())
	}

	pub async fn get_solver(&self) -> Result<DWaveAnnealerGenerator, ApiError> {
		(self.get_solvers().await? as Vec<DWaveAnnealerGenerator>)
			.into_iter()
			.next()
			.ok_or(ApiError::NotFound)
	}

	pub fn get_solvers_sync(&self) -> Result<Vec<DWaveAnnealerGenerator>, ApiError> {
		let mut runtime = tokio::runtime::Runtime::new().unwrap();
		runtime.block_on(self.get_solvers())
	}

	pub async fn get_solvers(&self) -> Result<Vec<DWaveAnnealerGenerator>, ApiError> {
		let mut v = self.inner.get_solvers(self.machine.as_deref()).await?;
		v.sort_by(|a, b| a.avg_load.partial_cmp(&b.avg_load).unwrap());
		Ok(v.into_iter()
			.filter(|sinfo| {
				if let Some(category) = self.category {
					if category != sinfo.properties.category {
						return false;
					}
				}
				true
			})
			.map(|sinfo| DWaveAnnealerGenerator::from_info(sinfo, self.inner.clone()))
			.collect::<Result<Vec<_>, ApiError>>()?)
	}
}

pub struct DWaveSession {
	pub timeout: Duration,
	pub endpoint: String,
	pub token: Option<String>,
	pub proxy: Option<String>,
	pub poll_backoff_min: Duration,
}

#[tokio::test]
async fn test_list_solvers() {
	let api = DWaveApi::from_file(Some(PathBuf::from("dwave.conf")), None).unwrap();
	let v = api.get_solvers().await.unwrap();
	for item in v.iter() {
		println!(
			"id = {}, status = {}, description = {}, avg_load = {}",
			item.info.id, item.info.status, item.info.description, item.info.avg_load
		);
	}
}

const CHUNK_SIZE: usize = 5 * 1024 * 1024;
impl DWaveSession {
	pub fn new() -> Self {
		Self::from_properties(None, None, None)
	}

	pub fn from_properties(
		endpoint: Option<String>,
		token: Option<String>,
		proxy: Option<String>,
	) -> Self {
		Self {
			timeout: Duration::new(60, 0),
			endpoint: endpoint.unwrap_or("https://cloud.dwavesys.com/sapi".to_owned()),
			token,
			proxy,
			poll_backoff_min: Duration::from_millis(50),
		}
	}

	builder_pattern!(token);
	builder_pattern!(proxy);

	async fn get_solvers(&self, machine: Option<&str>) -> Result<Vec<SolverInfo>, ApiError> {
		let session = self.create_session()?;
		let url = if let Some(machine) = machine {
			format!("{}/solvers/remote/{}/", self.endpoint, machine)
		} else {
			format!("{}/solvers/remote/", self.endpoint)
		};
		let resp = session.get(&url).send().await?;
		if !resp.status().is_success() {
			return Err(ApiError::Api(resp.status().as_str().to_owned()));
		}
		Ok(resp.json::<Vec<SolverInfo>>().await?)
	}

	fn create_session(&self) -> Result<Client, ApiError> {
		let mut builder = Client::builder();
		// Set stub user agent
		builder = builder.user_agent(
			"dwave-cloud-client/0.8.1 python/3.8.5 CPython/3.8.5-64bit machine/unknown system/unknown platform/unknown",
		);
		if let Some(token) = &self.token {
			let mut headers = HeaderMap::new();
			headers.insert(
				HeaderName::from_lowercase(b"x-auth-token").unwrap(),
				HeaderValue::from_str(&token).unwrap(),
			);
			builder = builder.default_headers(headers);
		}
		if let Some(proxy) = &self.proxy {
			builder = builder.proxy(Proxy::all(proxy)?)
		}
		Ok(builder.build()?)
	}

	// client.py:1096
	pub(crate) async fn submit_problem<'a>(
		&self,
		solver: &'a str,
		data: &'a ProblemData,
		problem_type: ProblemType,
		params: &'a HashMap<String, Value>,
	) -> Result<SolverAnswer, ApiError> {
		#[derive(Serialize)]
		struct RequestBody<'a> {
			// solver.py:356
			data: &'a ProblemData,
			#[serde(rename = "type")]
			problem_type: ProblemType,
			solver: &'a str,
			params: &'a HashMap<String, Value>,
		}
		#[derive(Deserialize)]
		struct Response {
			id: String,
			status: String,
			#[serde(default)]
			error_msg: Option<String>,
			#[serde(default)]
			error_message: Option<String>,
			#[serde(default)]
			answer: Option<SolverAnswer>,
		}
		let body = RequestBody {
			data,
			problem_type,
			solver,
			params,
		};
		let session = self.create_session()?;
		let mut resp = session
			.post(&format!("{}/problems/", &self.endpoint))
			.json(&[body])
			.send()
			.await?;
		loop {
			if !resp.status().is_success() {
				return Err(ApiError::Api(resp.status().as_str().to_owned()));
			}
			let result = resp
				.json::<Vec<Response>>()
				.await?
				.pop()
				.ok_or(ApiError::Api("Answer not found".to_owned()))?;
			if let Some(message) = result.error_msg {
				return Err(ApiError::Api(message.clone()));
			}
			match result.status.as_str() {
				"COMPLETED" => {
					if let Some(ans) = result.answer {
						return Ok(ans);
					} else {
						resp = session
							.get(&format!("{}/problems/{}/", &self.endpoint, &result.id))
							.send()
							.await?;
						// continue loop
					}
				}
				"CANCELLED" => {
					return Err(ApiError::Cancelled);
				}
				"IN_PROGRESS" | "PENDING" => {
					tokio::time::sleep(self.poll_backoff_min).await;
					resp = session
						.get(&format!("{}/problems/?id={}", &self.endpoint, &result.id))
						.send()
						.await?;
					// continue loop
				}
				_ => {
					return Err(ApiError::Api(
						result
							.error_message
							.unwrap_or("unknown error occured".to_owned()),
					));
				}
			}
		}
	}

	pub(crate) async fn upload_problem(
		&self,
		problem: &[u8],
		problem_id: Option<String>,
	) -> Result<String, ApiError> {
		let session = self.create_session()?;
		let problem_id = if let Some(problem_id) = problem_id {
			problem_id
		} else {
			self.initiate_multipart_upload(&session, problem.len())
				.await?
		};
		let uploaded_hashes = self.get_multipart_status(&session, &problem_id).await?;
		let mut uploading_hashes = HashMap::new();
		let parts = problem.len() / CHUNK_SIZE;
		let mut combined_hash = Vec::new();
		// TODO: parallel upload
		for part_no in 0..parts {
			let begin = part_no * CHUNK_SIZE;
			let end = std::cmp::min(begin + CHUNK_SIZE, problem.len());
			let part = &problem[begin..end];
			let hash = self
				.upload_multipart_part(
					&session,
					&problem_id,
					part_no,
					part,
					uploaded_hashes.get(&part_no).map(|a| a.as_ref()),
				)
				.await?;
			combined_hash.push(hash.clone());
			uploading_hashes.insert(part_no, hash);
		}
		// verify all parts uploaded
		let remote_hashes = self.get_multipart_status(&session, &problem_id).await?;
		for (part_no, hash) in uploading_hashes.iter() {
			let remote_hash = remote_hashes.get(part_no).ok_or(ApiError::Api(format!(
				"Cannot upload part {} of problem {}",
				&part_no, &problem_id
			)))?;
			if remote_hash != hash {
				return Err(ApiError::Api(format!(
					"Cannot upload part {} of problem {}: hash mismatch",
					&part_no, &problem_id
				)));
			}
		}
		// send parts combine request
		let combined_hash = format!(
			"{:x}",
			md5::compute(hex::decode(&combined_hash.join("")).unwrap())
		);
		self.combine_uploaded_parts(&session, &problem_id, &combined_hash)
			.await?;
		Ok(problem_id)
	}

	async fn upload_multipart_part(
		&self,
		session: &Client,
		problem_id: &str,
		part_no: usize,
		chunk: &[u8],
		uploaded_checksum: Option<&str>,
	) -> Result<String, ApiError> {
		let digest = md5::compute(chunk);
		let hexdigest = format!("{:x}", &digest);
		if let Some(checksum) = uploaded_checksum {
			if hexdigest == checksum {
				return Ok(hexdigest);
			}
		}
		let b64digest = base64::encode(*digest);
		let resp = session
			.put(&format!(
				"{}/bqm/multipart/{}/part/{}",
				&self.endpoint, problem_id, part_no
			))
			.header("Content-MD5", b64digest)
			.header("Content-Type", "application/octet-stream")
			.body(chunk.to_vec()) // TODO: more effecient
			.send()
			.await?;
		if !resp.status().is_success() {
			return Err(ApiError::Api(resp.status().as_str().to_owned()));
		}
		Ok(hexdigest)
	}

	/// Returns problem id
	async fn initiate_multipart_upload(
		&self,
		session: &Client,
		size: usize,
	) -> Result<String, ApiError> {
		let mut body = HashMap::new();
		body.insert("size", format!("{}", size));
		let resp = session
			.post(&format!("{}/bqm/multipart", &self.endpoint))
			.json(&body)
			.send()
			.await?;
		if !resp.status().is_success() {
			return Err(ApiError::Api(resp.status().as_str().to_owned()));
		}
		let result = resp.json::<HashMap<String, String>>().await?;
		result
			.get("id")
			.cloned()
			.ok_or(ApiError::Api("problem ID missing".to_owned()))
	}

	async fn combine_uploaded_parts(
		&self,
		session: &Client,
		problem_id: &str,
		checksum: &str,
	) -> Result<(), ApiError> {
		let mut body = HashMap::new();
		body.insert("checksum", checksum);
		let resp = session
			.post(&format!(
				"{}/bqm/multipart/{}/combine",
				&self.endpoint, problem_id
			))
			.json(&body)
			.send()
			.await?;
		if !resp.status().is_success() {
			return Err(ApiError::Api(resp.status().as_str().to_owned()));
		}
		Ok(())
	}

	/// Returns part number and checksum
	async fn get_multipart_status(
		&self,
		session: &Client,
		problem_id: &str,
	) -> Result<HashMap<usize, String>, ApiError> {
		#[derive(Deserialize)]
		struct MultipartStatusResultPart {
			part_number: usize,
			checksum: String,
		}
		#[derive(Deserialize)]
		struct MultipartStatusResult {
			error_msg: String,
			status: String,
			parts: Vec<MultipartStatusResultPart>,
		}
		let resp = session
			.get(&format!(
				"{}/bqm/multipart/{}/status",
				&self.endpoint, problem_id
			))
			.send()
			.await?;
		if !resp.status().is_success() {
			return Err(ApiError::Api(resp.status().as_str().to_owned()));
		}
		let status = resp.status();
		let result = resp.json::<MultipartStatusResult>().await?;
		if status.as_u16() != 200 {
			return Err(ApiError::Api(format!("{}", result.error_msg)));
		}
		let mut ret = HashMap::new();
		if result.status == "UPLOAD_IN_PROGRESS" {
			for part in result.parts.iter() {
				ret.insert(
					part.part_number,
					part.checksum.trim_matches(&['"'] as &[_]).to_owned(),
				);
			}
		}
		Ok(ret)
	}
}
