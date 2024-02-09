VERSION 0.8
PROJECT blue-build/cli

IMPORT github.com/blue-build/earthly-lib/cargo AS cargo

ARG --global IMAGE=ghcr.io/blue-build/cli

all:
	BUILD +build
	BUILD ./integration-tests+all --NIGHTLY=true --NIGHTLY=false

build:
	BUILD +default
	BUILD +nightly

default:
	ARG NIGHTLY=false
	WAIT
		BUILD +lint --NIGHTLY=$NIGHTLY
		BUILD +test --NIGHTLY=$NIGHTLY
	END
	BUILD +blue-build-cli --NIGHTLY=$NIGHTLY
	BUILD +blue-build-cli-alpine --NIGHTLY=$NIGHTLY
	BUILD +installer --NIGHTLY=$NIGHTLY

nightly:
	BUILD +default --NIGHTLY=true

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

	SAVE ARTIFACT target/$BUILD_TARGET/release/bluebuild

common:
	FROM ghcr.io/blue-build/earthly-lib/cargo-builder

	WORKDIR /app
	COPY --keep-ts --dir src/ templates/ /app
	COPY --keep-ts Cargo.* /app
	COPY --keep-ts *.md /app
	COPY --keep-ts LICENSE /app
	COPY --keep-ts build.rs /app

	DO cargo+INIT

blue-build-cli:
	FROM registry.fedoraproject.org/fedora-toolbox
	ARG NIGHTLY=false

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY

	RUN dnf install --refresh -y buildah podman skopeo

	COPY +cosign/cosign /usr/bin/cosign

	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY) /usr/bin/bluebuild

	ARG TAG
	ARG LATEST=false
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --NIGHTLY=$NIGHTLY

blue-build-cli-alpine:
	FROM alpine
	ARG NIGHTLY=false

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-musl" --NIGHTLY=$NIGHTLY

	RUN apk update && apk add buildah podman skopeo fuse-overlayfs

	COPY +cosign/cosign /usr/bin/cosign
	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-musl" --NIGHTLY=$NIGHTLY) /usr/bin/bluebuild

	ARG TAG
	ARG LATEST=false
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --NIGHTLY=$NIGHTLY --ALPINE=true

installer:
	FROM alpine
	ARG NIGHTLY=false

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY
	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-gnu" --NIGHTLY=$NIGHTLY) /out/bluebuild
	COPY install.sh /install.sh

	CMD ["cat", "/install.sh"]

	ARG TAG
	ARG LATEST=false
	ARG INSTALLER=true
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --NIGHTLY=$NIGHTLY --INSTALLER=$INSTALLER

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign
