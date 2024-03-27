VERSION 0.8
PROJECT blue-build/cli

IMPORT github.com/blue-build/earthly-lib/cargo AS cargo

ARG --global IMAGE=ghcr.io/blue-build/cli

all:
	BUILD +build
	BUILD ./integration-tests+all

build:
	WAIT
		BUILD +exports-script
	END
	BUILD +lint
	BUILD +test
	BUILD +blue-build-cli
	BUILD +blue-build-cli-alpine
	BUILD +installer

lint:
	FROM +common
	DO cargo+LINT

test:
	FROM +common
	DO cargo+TEST

install:
	FROM +common

	ARG --required BUILD_TARGET

	DO cargo+BUILD_RELEASE --BUILD_TARGET=$BUILD_TARGET

	SAVE ARTIFACT target/$BUILD_TARGET/release/bluebuild

exports-script:
	FROM alpine
	LABEL org.opencontainers.image.source="https://github.com/blue-build/cli"
	COPY exports.sh /
	RUN chmod +x exports.sh
	SAVE IMAGE --push $IMAGE:exports

common:
	FROM ghcr.io/blue-build/earthly-lib/cargo-builder

	WORKDIR /app
	COPY --keep-ts --dir src/ template/ recipe/ utils/ /app
	COPY --keep-ts Cargo.* /app
	COPY --keep-ts *.md /app
	COPY --keep-ts LICENSE /app
	COPY --keep-ts build.rs /app

	DO cargo+INIT

blue-build-cli:
	FROM registry.fedoraproject.org/fedora-toolbox

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-gnu"

	RUN dnf -y install dnf-plugins-core \
		&& dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo \
		&& dnf install --refresh -y \
			jq \
			docker-ce \
			docker-ce-cli \
			containerd.io \
			docker-buildx-plugin \
			docker-compose-plugin \
			buildah \
			podman \
			skopeo

	COPY +cosign/cosign /usr/bin/cosign

	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-gnu") /usr/bin/bluebuild

	ARG TAG
	ARG LATEST=false

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	ENTRYPOINT ["bluebuild"]

	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST

blue-build-cli-alpine:
	FROM alpine

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-musl"

	RUN apk update && apk add buildah podman skopeo fuse-overlayfs

	COPY +cosign/cosign /usr/bin/cosign
	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-musl") /usr/bin/bluebuild

	ARG TAG
	ARG LATEST=false

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	ENTRYPOINT ["bluebuild"]

	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --ALPINE=true

installer:
	FROM alpine

	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-musl") /out/bluebuild
	COPY install.sh /install.sh

	CMD ["cat", "/install.sh"]

	ARG TAG
	ARG LATEST=false
	DO cargo+SAVE_IMAGE --IMAGE=$IMAGE --TAG=$TAG --LATEST=$LATEST --INSTALLER=true

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign
