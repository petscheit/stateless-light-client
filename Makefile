setup:
	./scripts/setup.sh

activate:
	@echo "Please source the virtual environment activation script:"
	@echo "  source scripts/activate.sh"

build-cairo:
	./scripts/cairo_compile.sh cairo/src/recursive_update.cairo

get-program-hash:
	# @make build
	@echo "RecursiveUpdateProgramHash:"
	@cairo-hash-program --program cairo/build/recursive_update.json