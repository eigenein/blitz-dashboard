.PHONY: tag/%
tag/%:
	command git tag --annotate $* -m $*
	command git push origin $*
	command git tag --force --annotate latest --message latest
	command git push --force origin latest
