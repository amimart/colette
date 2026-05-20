# Constants:
TARGET_FOLDER       = target

BOLD   := \033[1m
COLOR_RESET  = $(call get_color,sgr0,)
COLOR_CYAN   = $(call get_color,setaf,6)
COLOR_GREEN  = $(call get_color,setaf,2)
COLOR_YELLOW = $(call get_color,setaf,3)

# Some colors (if supported)
define get_color
$(shell tput -Txterm $(1) $(2) 2>/dev/null || echo "")
endef

all: help

## Build:
build: build-rust ## Build rust (i.e. not docker)

build-rust: ## Build the magnarr binary
	@printf "$(COLOR_GREEN)$(BOLD)🔨 Building...$(COLOR_RESET)\n"
	cargo build

lint-rust: ## Lint Rust code with Clippy
	@printf "$(COLOR_CYAN)$(BOLD)🦀 Linting Rust...$(COLOR_RESET)\n"
	cargo clippy -- -D warnings

lint-rust-format: ## Lint Rust formatting
	@printf "$(COLOR_CYAN)$(BOLD)🎨 Checking formatting...$(COLOR_RESET)\n"
	cargo fmt --check

lint-md: ## Lint Markdown
	@printf "$(COLOR_CYAN)$(BOLD)📝 Linting Markdown...$(COLOR_RESET)\n"
	npx markdownlint-cli2 "**/*.md"

## Clean:
clean: ## Clean all build artifacts and local deployment
	rm -rf $(TARGET_FOLDER)

## Help:
help: ## Show this help.
	@echo ''
	@echo 'Usage:'
	@echo '  ${COLOR_YELLOW}make${COLOR_RESET} ${COLOR_GREEN}<target>${COLOR_RESET}'
	@echo ''
	@echo 'Targets:'
	@$(foreach V,$(sort $(.VARIABLES)), \
		$(if $(filter-out environment% default automatic,$(origin $V)), \
			$(if $(filter TOOL_%,$V), \
				export $V="$($V)";))) \
	awk 'BEGIN {FS = ":.*?## "} { \
		if (/^[a-zA-Z_-]+:.*?##.*$$/) {printf "    ${COLOR_YELLOW}%-20s${COLOR_GREEN}%s${COLOR_RESET}\n", $$1, $$2} \
		else if (/^## .*$$/) {printf "  ${COLOR_CYAN}%s${COLOR_RESET}\n", substr($$1,4)} \
		}' $(MAKEFILE_LIST) | envsubst
