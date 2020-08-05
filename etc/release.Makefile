# TODO:
# - Update binary assets from bat
# - Update README prior to release
# - Update help text in README, with BAT_THEME unset

release: \
	clean \
	check_environment \
	bump-version \
	create-github-release \
	bump-version-in-documentation-links \
	bump-private-homebrew-formula \
	bump-public-homebrew-formula \
	publish-to-cargo


clean:
	rm -fr .make-sentinels


check_environment:
	[ -n  "$$DELTA_OLD_VERSION" ]
	[ -n  "$$DELTA_NEW_VERSION" ]
	mkdir -p .make-sentinels
	@echo "Release: $$DELTA_OLD_VERSION => $$DELTA_NEW_VERSION"


BUMP_VERSION_SENTINEL=.make-sentinels/bump-version
bump-version: $(BUMP_VERSION_SENTINEL)
$(BUMP_VERSION_SENTINEL):
	@echo Bumping version in Cargo.toml
	sed -i -E "s,^version = \"$$DELTA_OLD_VERSION\",version = \"$$DELTA_NEW_VERSION\",g" Cargo.toml
	make build
	make test
	git add Cargo.toml Cargo.lock
	git commit -m "Bump version" || true
	touch $(BUMP_VERSION_SENTINEL)


CREATE_GITHUB_RELEASE_SENTINEL=.make-sentinels/create-github-release
create-github-release: $(CREATE_GITHUB_RELEASE_SENTINEL)
$(CREATE_GITHUB_RELEASE_SENTINEL):
	which gren > /dev/null
	@echo \# Creating release tag
	git tag "$$DELTA_NEW_VERSION"
	git push
	git push --tags
	@echo \# Draft and edit release notes in Github
	gren release "$$DELTA_NEW_VERSION"
	@echo \# Wait for assets to appear at https://github.com/dandavison/delta/releases
	touch $(CREATE_GITHUB_RELEASE_SENTINEL)


BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL=.make-sentinels/bump-version-in-documentation-links
bump-version-in-documentation-links: $(BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL)
$(BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL):
	sed -i -E "s,$$DELTA_OLD_VERSION,$$DELTA_NEW_VERSION,g" README.md
	git add README.md
	git commit -m "Bump version in links to executables"
	touch $(BUMP_VERSION_IN_DOCUMENTATION_LINKS_SENTINEL)


BUMP_PRIVATE_HOMEBREW_FORMULA_SENTINEL=.make-sentinels/bump-private-homebrew-formula
bump-private-homebrew-formula: $(BUMP_PRIVATE_HOMEBREW_FORMULA_SENTINEL)
$(BUMP_PRIVATE_HOMEBREW_FORMULA_SENTINEL):
	sed -i -E "s,$$DELTA_OLD_VERSION,$$DELTA_NEW_VERSION,g" HomeBrewFormula/git-delta.rb
	make hash
	@echo \# modify hashes in HomeBrewFormula/git-delta.rb
	git add HomeBrewFormula/git-delta.rb
	git commit -m "Bump version in private Homebrew formula"
	touch $(BUMP_PRIVATE_HOMEBREW_FORMULA_SENTINEL)


BUMP_PUBLIC_HOMEBREW_FORMULA_SENTINEL=.make-sentinels/bump-public-homebrew-formula
bump-public-homebrew-formula: $(BUMP_PUBLIC_HOMEBREW_FORMULA_SENTINEL)
$(BUMP_PUBLIC_HOMEBREW_FORMULA_SENTINEL):
	make -f release.Makefile test-public-homebrew-formula
	cd "$$(brew --repo homebrew/core)" && brew bump-formula-pr --url "https://github.com/dandavison/delta/archive/$$DELTA_NEW_VERSION.tar.gz" git-delta
	touch $(BUMP_PUBLIC_HOMEBREW_FORMULA_SENTINEL)


test-public-homebrew-formula:
	cd $$(brew --repo homebrew/homebrew-core) && \
	brew uninstall --force git-delta && \
	brew install --build-from-source git-delta && \
	brew test git-delta && \
	brew uninstall --force git-delta && \
	brew install git-delta && \
	brew audit --strict git-delta


PUBLISH_TO_CARGO_SENTINEL=.make-sentinels/publish-to-cargo
publish-to-cargo: $(PUBLISH_TO_CARGO_SENTINEL)
$(PUBLISH_TO_CARGO_SENTINEL):
	cargo publish
	touch $(PUBLISH_TO_CARGO_SENTINEL)


.PHONY: \
	clean \
	release	\
	check_environment \
	bump-version \
	create-github-release \
	bump-version-in-documentation-links \
	bump-private-homebrew-formula \
	bump-public-homebrew-formula \
	test-public-homebrew-formula \
	publish-to-cargo
