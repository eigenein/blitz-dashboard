tankopedia.json:
	@mv src/tankopedia.json src/tankopedia.json.backup
	@curl -s "https://api.wotblitz.ru/wotb/encyclopedia/vehicles/?application_id=${APP_ID}" >> src/tankopedia.json.backup
	@jq -s '.[0] * .[1].data' src/tankopedia.json.backup > src/tankopedia.json
	@rm src/tankopedia.json.backup
