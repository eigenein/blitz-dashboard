tankopedia.json:
	@mv tankopedia.json tankopedia.json.backup
	@curl -s "https://api.wotblitz.ru/wotb/encyclopedia/vehicles/?application_id=${APP_ID}" >> tankopedia.json.backup
	@jq -s '.[0] * .[1].data' tankopedia.json.backup > tankopedia.json
	@rm tankopedia.json.backup
