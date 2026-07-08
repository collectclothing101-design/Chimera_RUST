# =============================================================
# Chimera RUST — Makefile
# =============================================================
.DEFAULT_GOAL := check

PROFILE  ?= debug
ARCH     ?= x86_64
SIGN_ID  ?=

CARGO    := cargo
SCRIPTS  := scripts

# ── Development ───────────────────────────────────────────────
.PHONY: check
check:
	$(CARGO) check --workspace

.PHONY: build
build:
	$(CARGO) build --workspace $(if $(filter release,$(PROFILE)),--release,)

.PHONY: test
test:
	$(CARGO) test --workspace

.PHONY: clippy
clippy:
	$(CARGO) clippy --workspace -- \
	  -D warnings \
	  -A clippy::too_many_arguments \
	  -A clippy::module_inception

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check:
	$(CARGO) fmt --all -- --check

# ── Fix application ───────────────────────────────────────────
.PHONY: apply-fixes
apply-fixes:
	@bash $(SCRIPTS)/apply_fixes.sh

# ── macOS .app bundle ─────────────────────────────────────────
.PHONY: app
app:
	@bash $(SCRIPTS)/build_app.sh \
	  --arch $(ARCH) \
	  $(if $(filter release,$(PROFILE)),--release,) \
	  $(if $(SIGN_ID),--sign "$(SIGN_ID)",)

.PHONY: app-release
app-release: PROFILE=release
app-release: app

.PHONY: app-universal
app-universal:
	@bash $(SCRIPTS)/build_app.sh \
	  --arch universal \
	  --release \
	  $(if $(SIGN_ID),--sign "$(SIGN_ID)",)

# ── Icons ────────────────────────────────────────────────────
.PHONY: icons
icons:
	python3 $(SCRIPTS)/gen_icons.py assets/AppIcon.icns

# ── Dist / clean ─────────────────────────────────────────────
.PHONY: clean
clean:
	$(CARGO) clean
	rm -rf dist/

.PHONY: dist-clean
dist-clean: clean
	rm -rf target/

# ── Quick status ─────────────────────────────────────────────
.PHONY: status
status:
	@echo "── Workspace crates ──────────────────────────"
	@$(CARGO) metadata --no-deps --format-version 1 \
	  | python3 -c \
	  "import json,sys; \
	   m=json.load(sys.stdin); \
	   [print(f'  {p[\"name\"]:30s} v{p[\"version\"]}') \
	    for p in sorted(m['packages'],key=lambda x:x['name'])]"
	@echo ""
	@echo "── Toolchain ──────────────────────────────────"
	@rustc --version
	@cargo --version
