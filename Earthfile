VERSION \
	--global-cache \
	--use-function-keyword \
	--arg-scope-and-set \
	0.7

IMPORT gitlab.com/wunker-bunker/ci-pipelines/earthly/cargo AS cargo

ARG --global IMAGE=registry.gitlab.com/wunker-bunker/blue-build

all:
	BUILD +default
	BUILD +nightly

default:
	WAIT
		BUILD +lint
		BUILD +test
	END
	BUILD +blue-build-cli
	BUILD +blue-build-cli-alpine
	BUILD +installer

nightly:
	WAIT
		BUILD +lint --NIGHTLY=true
		BUILD +test --NIGHTLY=true
	END
	BUILD +blue-build-cli --NIGHTLY=true
	BUILD +blue-build-cli-alpine --NIGHTLY=true
	BUILD +installer --NIGHTLY=true

lint:
	FROM +common

	ARG NIGHTLY=false

	DO cargo+LINT --NIGHTLY=$NIGHTLY

test:
	FROM +common

	ARG NIGHTLY=false

	DO cargo+TEST --NIGHTLY=$NIGHTLY

install:
	FROM +common

	ARG NIGHTLY=false
	ARG --required BUILD_TARGET

	DO cargo+BUILD_RELEASE --BUILD_TARGET=$BUILD_TARGET --NIGHTLY=$NIGHTLY

	SAVE ARTIFACT target/$BUILD_TARGET/release/bb

common:
	FROM registry.gitlab.com/wunker-bunker/cargo-builder

	WORKDIR /app
	COPY --keep-ts --dir src/ templates/ /app
	COPY --keep-ts Cargo.* /app
	COPY --keep-ts *.md /app
	COPY --keep-ts LICENSE /app

	DO cargo+INIT

blue-build-cli:
	FROM registry.fedoraproject.org/fedora-toolbox
	ARG NIGHTLY=false

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY

	RUN dnf install --refresh -y buildah podman skopeo

	COPY +cosign/cosign /usr/bin/cosign

	COPY (+install/bb --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY) /usr/bin/bb

	ARG TAG
	ARG LATEST=false
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --NIGHTLY=$NIGHTLY

blue-build-cli-alpine:
	FROM alpine
	ARG NIGHTLY=false

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-musl" --NIGHTLY=$NIGHTLY

	RUN apk update && apk add buildah podman skopeo fuse-overlayfs

	COPY +cosign/cosign /usr/bin/cosign
	COPY (+install/bb --BUILD_TARGET="x86_64-unknown-linux-musl" --NIGHTLY=$NIGHTLY) /usr/bin/bb

	ARG TAG
	ARG LATEST=false
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --NIGHTLY=$NIGHTLY --ALPINE=true

installer:
	FROM alpine
	ARG NIGHTLY=false

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY

	COPY (+install/bb --BUILD_TARGET="x86_64-unknown-linux-gnu") /out/bb
	COPY install.sh /install.sh

	CMD ["cat", "/install.sh"]

	ARG TAG
	ARG LATEST=false
	ARG INSTALLER=true
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --NIGHTLY=$NIGHTLY --INSTALLER=$INSTALLER

iso-generator:
	FROM registry.fedoraproject.org/fedora-toolbox

    GIT CLONE https://github.com/ublue-os/isogenerator.git /isogenerator
    WORKDIR /isogenerator
    ARG PACKAGES=$(cat deps.txt)
    RUN dnf install --disablerepo="*" --enablerepo="fedora,updates" --setopt install_weak_deps=0 --assumeyes $PACKAGES

    SAVE IMAGE --push $IMAGE/iso-generator

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign
