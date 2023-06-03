.PHONY:	clean
clean:
	rm -rf Cargo.lock .mypy_cache .venv target *.egg-info rustqubo/*.so rustqubo/*.dylib rustqubo/*.dll
