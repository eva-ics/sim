all:
	@echo "Select target"

ver:
	./dev/update-version.py

release:
	ansible-playbook -i lab-builder1, ./dev/update.yml
