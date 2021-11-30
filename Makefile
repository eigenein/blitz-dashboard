.PHONY: tag/%
tag/%:
	command git tag -a $* -m $*
	command git push origin $*

.PHONY: bench
bench:
	command cargo +nightly bench

.PHONY: format
format:
	command cargo +nightly fmt
