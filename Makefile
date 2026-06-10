FLOW_GIT_URL ?= https://github.com/oharlem/flow

.PHONY: up install-git install-v0.1.0 patch minor

up:
	cargo install --path crates/flow-cli --locked --force
	flow --version
	flow update
	flow doctor

install-git:
	cargo install --git $(FLOW_GIT_URL) --locked --force flow-cli
	flow --version

install-v0.1.0:
	cargo install --git $(FLOW_GIT_URL) --tag v0.1.0 --locked --force flow-cli
	flow --version

patch:
	./scripts/bump-patch.sh

minor:
	./scripts/bump-minor.sh
