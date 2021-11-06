.PHONY: tag/%
tag/%:
	command git tag -a $* -m $*
	command git push origin $*
