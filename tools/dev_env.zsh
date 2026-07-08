#!/usr/bin/env zsh
# =============================================================================
# macOS / Linux Development Environment – Complete PATH & Tools
# =============================================================================
# ChimeraRS development environment configuration
# Source this file: source tools/dev_env.zsh

[[ $- != *i* ]] && return

# ── 1. System & Homebrew ───────────────────────────────────────────────
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"

if [[ -f /opt/homebrew/bin/brew ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
elif [[ -f /usr/local/bin/brew ]]; then
    eval "$(/usr/local/bin/brew shellenv)"
fi

# ── 2. Python ──────────────────────────────────────────────────────────
for ver in 3.15 3.14 3.10; do
    [[ -d "/Library/Frameworks/Python.framework/Versions/$ver/bin" ]] && \
        export PATH="/Library/Frameworks/Python.framework/Versions/$ver/bin:$PATH"
done

# ── 3. Node.js (NVM) ──────────────────────────────────────────────────
export NVM_DIR="$HOME/.nvm"
[[ -s "$NVM_DIR/nvm.sh" ]] && source "$NVM_DIR/nvm.sh"
[[ -s "$NVM_DIR/bash_completion" ]] && source "$NVM_DIR/bash_completion"

# ── 4. Java ────────────────────────────────────────────────────────────
if [[ "$(uname)" == "Darwin" ]] && command -v /usr/libexec/java_home &>/dev/null; then
    export JAVA_HOME=$(/usr/libexec/java_home)
else
    export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/default-java}"
fi
export PATH="$JAVA_HOME/bin:$PATH"

# ── 5. Rust / Cargo ───────────────────────────────────────────────────
export PATH="$HOME/.cargo/bin:$PATH"

# ── 6. MacPorts ────────────────────────────────────────────────────────
export PATH="/opt/local/bin:/opt/local/sbin:$PATH"

# ── 7. ChimeraRS Development Paths ─────────────────────────────────────
export CHIMERA_HOME="$HOME/Chimera_RUST"
export CHIMERA_TOOLS="$CHIMERA_HOME/tools"
[[ -d "$CHIMERA_HOME" ]] && export PATH="$CHIMERA_HOME/deploy:$PATH"

# ── 8. Android SDK ─────────────────────────────────────────────────────
[[ -d "$HOME/Library/Android/sdk/platform-tools" ]] && \
    export PATH="$HOME/Library/Android/sdk/platform-tools:$PATH"
[[ -d "$HOME/Library/Android/sdk/tools" ]] && \
    export PATH="$HOME/Library/Android/sdk/tools:$PATH"

# =============================================================================
# ChimeraRS Development Commands
# =============================================================================

# Quick build
chimera-build() {
    cd "$CHIMERA_HOME" && ./deploy/build_app.sh --universal --no-sign
}

# Quick test
chimera-test() {
    cd "$CHIMERA_HOME" && cargo test -p chimera-ffi --lib
}

# Create DMG
chimera-dmg() {
    cd "$CHIMERA_HOME" && ./deploy/package_dmg.sh
}

# Publish release
chimera-release() {
    local ver="${1:-}"
    [[ -z "$ver" ]] && { echo "Usage: chimera-release <version>"; return 1; }
    cd "$CHIMERA_HOME" && ./deploy/release.sh --version "$ver" --skip-build
}

# Run mock ADB server
chimera-mock() {
    cd "$CHIMERA_HOME" && python3 tools/mock_adb_server.py
}

# Check workspace
chimera-check() {
    cd "$CHIMERA_HOME" && cargo check --workspace
}

# Open the built app
chimera-run() {
    open "$CHIMERA_HOME/target/debug/Chimera.app"
}

# Quick device scan (requires ADB)
chimera-scan() {
    adb devices -l 2>/dev/null || echo "ADB not running. Start with: adb start-server"
}

# =============================================================================
# AI Command Suite
# =============================================================================

ai-mem-edit() {
    local mem_dir="$PWD/.claude-memory"
    mkdir -p "$mem_dir"
    local mem_file="$mem_dir/remember.md"
    [[ ! -f "$mem_file" ]] && echo "# Project Memory for $PWD" > "$mem_file"
    "${EDITOR:-nano}" "$mem_file"
}

deep-research() {
    local query="${1:-}"
    [[ -z "$query" ]] && { echo "Usage: deep-research <query>"; return 1; }
    echo "Running Deep Research: $query"
}

alias cot-prompt='print -z "Think step-by-step. Break the problem into logical parts, solve each, then synthesize a final answer."'
alias tot-prompt='print -z "Explore multiple approaches (branch A, B, C). List pros/cons for each, then pick the best one and justify."'
alias think-deep='print -z "Activate Extended Thinking Mode: spend up to 2 minutes reasoning through this, explore edge cases, and provide a final, well-justified answer."'

ai-style() {
    local persona="${1:-professional}"
    case "$persona" in
        professional|email)   echo "Style: Team Email – concise, neutral, action-oriented." ;;
        linkedin|social)      echo "Style: LinkedIn Post – engaging, storytelling, hashtags." ;;
        technical|docs)       echo "Style: Technical Docs – precise, structured, code-friendly." ;;
        *)                    echo "Unknown persona. Use: professional, linkedin, technical." ;;
    esac
}

ai-cli() {
    local desc="${1:-}"
    [[ -z "$desc" ]] && { echo "Usage: ai-cli \"<command description>\""; return 1; }
    echo "AI will generate the exact CLI command for: $desc"
}

ai-scaffold() {
    local type="${1:-web}"
    local name="${2:-my-project}"
    mkdir -p "$name"
    cat > "$name/README.md" <<EOF
# ${name} – AI-Assisted Project

Goal: Brief description here.
AI Prompts Used: ... (track your prompts for reproducibility)
Steps:
1. Setup environment
2. Run \`ai-cli "install dependencies"\`
3. ...
EOF
    echo "Scaffolded $type project in $name/"
}

ai-decompose() {
    local task="${1:-}"
    [[ -z "$task" ]] && { echo "Usage: ai-decompose \"<task>\""; return 1; }
    echo "Decomposing: $task → sub-tasks will be listed."
}

ai-optimize() {
    local prompt="${1:-}"
    [[ -z "$prompt" ]] && { echo "Usage: ai-optimize \"<prompt>\""; return 1; }
    echo "Optimizing prompt: $prompt"
}

ai-review() {
    local file="${1:-}"
    [[ -f "$file" ]] || { echo "Usage: ai-review <file>"; return 1; }
    echo "Reviewing $file for bugs, style, and security..."
}

ai-tests() {
    local file="${1:-}"
    [[ -f "$file" ]] || { echo "Usage: ai-tests <file>"; return 1; }
    echo "Generating tests for $file..."
}

ai-docs() {
    local file="${1:-}"
    [[ -f "$file" ]] || { echo "Usage: ai-docs <file>"; return 1; }
    echo "Generating docs for $file..."
}

# =============================================================================
# LLM Code Verification Checklist
# =============================================================================

llm-checklist() {
    cat <<'EOF'
**LLM-Generated Code Verification Checklist**
1. Verify all packages, commands, and configurations by official docs.
2. Remove LLM noise: delete useless/silly comments; keep comments that explain *why*.
3. Check package versions – LLMs may suggest outdated APIs.
4. Ensure consistent naming – new files match project conventions.
5. Understand & explain the code – you should be able to articulate trade-offs.
EOF
}

code-review-3pass() {
    cat <<'EOF'
**Code Review 3-Pass Method**
Pass 1: What & Why (2 mins) – Read file list, linked ticket, PR description.
Pass 2: Correctness (10-20 mins) – Logic errors, error handling, concurrency.
Pass 3: Details (5-10 mins) – Naming, formatting, comments.
EOF
}

# =============================================================================
# Summary
# =============================================================================

ai-summary() {
    cat <<'SUMMARY'
ChimeraRS Dev Commands:
  chimera-build    – Build universal app
  chimera-test     – Run FFI tests
  chimera-dmg      – Create DMG
  chimera-release  – Publish to GitHub
  chimera-mock     – Start mock ADB server
  chimera-check    – Workspace check
  chimera-run      – Open built app
  chimera-scan     – Scan USB/ADB devices

AI Commands:
  ai-mem-edit, deep-research, ai-cli, ai-scaffold
  ai-decompose, ai-optimize, ai-review, ai-tests, ai-docs
  cot-prompt, tot-prompt, think-deep
  llm-checklist, code-review-3pass
SUMMARY
}

# ── Final validation ───────────────────────────────────────────────────
if [[ -o interactive ]]; then
    echo "ChimeraRS dev environment loaded"
    echo "  Rust:    $(command -v cargo 2>/dev/null || echo 'not installed')"
    echo "  Python:  $(command -v python3 2>/dev/null || echo 'not installed')"
    echo "  ADB:     $(command -v adb 2>/dev/null || echo 'not installed')"
    echo "  Type 'ai-summary' for all commands"
fi
