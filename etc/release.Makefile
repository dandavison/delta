# TODO:
# - Check for a bat upgrade as it might bring new language support/themes
# - Update README prior to release
# - Update help text in README, with BAT_THEME unset

release: \
	clean \
	check-environment \
	bump-version \
	create-github-release \
	bump-version-in-documentation-links


clean:
	rm -fr .make-sentinels


check-environment:
	[ -n  "$$DELTA_OLD_VERSION" ]
	[ -n  "$$DELTA_NEW_VERSION" ]
	mkdir -p .make-sentinels
	@echo "Release: $$DELTA_OLD_VERSION => $$DELTA_NEW_VERSION"


BUMP_VERSION_SENTINEL=.make-sentinels/bump-version
bump-version: $(BUMP_VERSION_SENTINEL)
$(BUMP_VERSION_SENTINEL):
	@echo Bumping version in Cargo.toml
	sed -i -E "s,^version = \"$$DELTA_OLD_VERSION\",version = \"$$DELTA_NEW_VERSION\",g" Cargo.toml
	cargo build --release
	git add Cargo.toml Cargo.lock
	git commit -m "Bump version" || true
	touch $(BUMP_VERSION_SENTINEL)


CREATE_GITHUB_RELEASE_SENTINEL=.make-sentinels/create-github-release
create-github-release: $(CREATE_GITHUB_RELEASE_SENTINEL) check-environment
$(CREATE_GITHUB_RELEASE_SENTINEL):
	@echo \# Creating release tag
	git tag "$$DELTA_NEW_VERSION"
	git push
	git push --tags
	@echo \# See https://github.com/dandavison/delta/releases
	touch $(CREATE_GITHUB_RELEASE_SENTINEL)


BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL=.make-sentinels/bump-version-in-documentation-links
bump-version-in-documentation-links: $(BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL)
$(BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL):
	sed -i -E "s,$$DELTA_OLD_VERSION,$$DELTA_NEW_VERSION,g" manual/src/full---help-output.md manual/src/installation.md
	rg -qF "$$DELTA_NEW_VERSION" manual/src/installation.md
	git add manual/src/full---help-output.md manual/src/installation.md
	git commit -m "Link to new binaries"
	touch $(BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL)


.PHONY: \
	clean \
	release	\
	check_environment \
	bump-version \
	create-github-release \
	bump-version-in-documentation-links
