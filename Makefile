build:
	cargo build --release --features cli --bin solo2
	ls -sh target/release/solo2


# for AUR things, kudos to <https://github.com/Foxboron/sbctl>

push-aur:
	cd pkg/arch; makepkg --printscr-info > .SRCINFO
	git subtree push -P pkg/arch

.PHONY: local-aur
.ONESHELL:
local-aur:
	cd pkg/arch
	mkdir -p ./src
	ln -srfT $(CURDIR) ./src/solo2-cli-0.0.7
	makepkg --holdver --syncdeps --noextract --force --install
