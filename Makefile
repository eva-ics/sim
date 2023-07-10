all:
	@echo "Select target"

ver:
	./dev/update-version.py

release:
	ansible-playbook -i lab-builder1, ./dev/update.yml
	ssh -t lab-builder1 ". ~/.cargo/env && cd /build/sim && git checkout main && ./dev/build-and-release.py"
