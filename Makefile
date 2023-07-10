all:
	@echo "Select target"

ver:
	./dev/update-version.py

release:
	ansible-playbook -i lab-builder1, ./dev/update.yml
	ssh -t lab-builder1 ". ~/.cargo/env && cd /build/sim && git checkout main && ./dev/build-and-release.py"
	rci job run pub.bma.ai

release-installer:
	gsutil -h "Cache-Control:no-cache" -h "Content-Type:text/x-shellscript" \
		cp -a public-read install.sh gs://pub.bma.ai/sim/install
	rci job run pub.bma.ai
