use std::path::{Path, PathBuf};
#[cfg(windows)]
pub fn get_dwave_path() -> Vec<PathBuf> {
	let mut v = Vec::new();
	// TODO: use SHGetFolderPathW()
	v.push(
		PathBuf::from("C:")
			.push("Documents and Settings")
			.push("All Users")
			.push("Application Data")
			.push("dwave")
			.push("dwave.conf"),
	);
	v.push(
		PathBuf::from("C:")
			.push("ProgramData")
			.push("dwave")
			.push("dwave.conf"),
	);
	v.push(
		PathBuf::from(shellexpand::tilde("~"))
			.push("Local Settings")
			.push("Application Data")
			.push("dwave")
			.push("dwave.conf"),
	);
	v.push(
		PathBuf::from(shellexpand::tilde("~"))
			.push("AppData")
			.push("Local")
			.push("dwave")
			.push("dwave.conf"),
	);
	v.push(env::current_dir().unwrap().push("dwave.conf"));
	v
}

#[cfg(target_os = "macos")]
pub fn get_dwave_path() -> Vec<PathBuf> {
	let mut v = Vec::new();
	v.push(
		PathBuf::from("/Library/Application Support")
			.push("dwave")
			.push("dwave.conf"),
	);
	v.push(
		PathBuf::from(shellexpand::tilde("~/Library/Application Support"))
			.push("dwave")
			.push("dwave.conf"),
	);
	v.push(Path::from("./dwave.conf").to_path_buf());
	v
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn get_dwave_path() -> Vec<PathBuf> {
	let mut v = std::env::split_paths(
		&shellexpand::tilde(
			std::env::var("XDG_CONFIG_DIRS")
				.as_ref()
				.unwrap_or(&"/etc/xdg".to_owned()),
		)
		.into_owned(),
	)
	.into_iter()
	.map(|path| {
		let mut buf =
			PathBuf::from(&shellexpand::tilde(path.as_path().to_str().unwrap()).into_owned());
		buf.push("dwave");
		buf.push("dwave.conf");
		buf
	})
	.collect::<Vec<_>>();
	v.push({
		let mut buf = PathBuf::from(
			shellexpand::tilde(
				std::env::var("XDG_CONFIG_HOME")
					.as_ref()
					.unwrap_or(&"~/.config".to_owned()),
			)
			.into_owned(),
		);
		buf.push("dwave");
		buf.push("dwave.conf");
		buf
	});
	v.push(Path::new("./dwave.conf").to_path_buf());
	v
}
