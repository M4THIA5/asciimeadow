CRATE  := asciimeadow
spec   ?= packaging/asciimeadow.spec
outdir ?= $(CURDIR)

VERSION := $(shell git describe --exact-match --tags HEAD 2>/dev/null | sed 's/^v//')
ifeq ($(VERSION),)
VERSION := $(shell git describe --tags --always 2>/dev/null | sed 's/^v//')
endif
ifeq ($(VERSION),)
VERSION := $(shell grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
endif
VERSION := $(subst -,.,$(VERSION))

srpm:
	command -v cargo >/dev/null 2>&1 || dnf -y install cargo
	cargo vendor vendor/
	staging=$$(mktemp -d) && \
	trap 'rm -rf "$$staging"' EXIT && \
	tar caf "$$staging/$(CRATE)-$(VERSION).tar.gz" \
		--exclude=./.git --exclude=./vendor --exclude='./*.tar.*' \
		--transform 's,^\.,$(CRATE)-$(VERSION),' . && \
	tar caf "$$staging/$(CRATE)-$(VERSION)-vendor.tar.xz" vendor/ && \
	rpmbuild -bs "$(spec)" \
		--define "_sourcedir $$staging" \
		--define "_srcrpmdir $(outdir)"

.PHONY: srpm
