.PHONY: help deploy-rc-app deploy-app deploy-landing version install-clients check-clients

SCRIPTS := utils/scripts

LANDING_DIST := apps/landing/dist/
DEPLOY_HOST  := eve-dev
DEPLOY_PATH  := /var/www/karto

help: ## Muestra esta ayuda
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

version: ## Imprime la versión actual (Cargo.toml)
	@grep -m1 -E '^version = ' apps/desktop/src-tauri/Cargo.toml | sed -E 's/^version = "(.*)"/\1/'

install-clients: ## Instala los clientes externos que Karto invoca en runtime
	@bash $(SCRIPTS)/install_clients.sh

check-clients: ## Reporta qué clientes externos faltan (no instala nada)
	@bash $(SCRIPTS)/install_clients.sh --check

deploy-rc-app: ## Publica un Release Candidate (bump + -rc.N, tag y push)
	@bash $(SCRIPTS)/deploy_rc.sh

deploy-app: ## Promueve el RC actual a release estable (quita -rc, tag y push)
	@bash $(SCRIPTS)/deploy_release.sh

deploy-landing: ## Compila la landing y la sincroniza a eve-dev:/var/www/karto vía rsync
	pnpm --filter @karto/landing build
	rsync -avz --delete --rsync-path="sudo rsync" --chown=www-data:www-data $(LANDING_DIST) $(DEPLOY_HOST):$(DEPLOY_PATH)/
