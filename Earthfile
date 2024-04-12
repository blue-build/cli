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

	ARG EARTHLY_GIT_SHORT_HASH
	SAVE IMAGE --push $IMAGE:$EARTHLY_GIT_SHORT_HASH-exports

common:
	FROM ghcr.io/blue-build/earthly-lib/cargo-builder

	WORKDIR /app
	COPY --keep-ts --dir src/ template/ recipe/ utils/ /app
	COPY --keep-ts Cargo.* /app
	COPY --keep-ts *.md /app
	COPY --keep-ts LICENSE /app
	COPY --keep-ts build.rs /app
	COPY --dir .git/ /app

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

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	ENTRYPOINT ["bluebuild"]

	ARG TAG
	ARG LATEST=false

	IF [ -n "$TAG" ]
		SAVE IMAGE --push $IMAGE:$TAG

		IF [ "$LATEST" = "true" ]
			SAVE IMAGE --push $IMAGE:latest
		END
	ELSE
		ARG EARTHLY_GIT_BRANCH
		SAVE IMAGE --push $IMAGE:$EARTHLY_GIT_BRANCH
	END

blue-build-cli-alpine:
	FROM alpine

	BUILD +install --BUILD_TARGET="x86_64-unknown-linux-musl"

	RUN apk update && apk add buildah podman skopeo fuse-overlayfs

	COPY +cosign/cosign /usr/bin/cosign
	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-musl") /usr/bin/bluebuild

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	ENTRYPOINT ["bluebuild"]

	ARG TAG
	ARG LATEST=false

	IF [ -n "$TAG" ]
		SAVE IMAGE --push $IMAGE:$TAG-alpine

		IF [ "$LATEST" = "true" ]
			SAVE IMAGE --push $IMAGE:latest-alpine
		END
	ELSE
		ARG EARTHLY_GIT_BRANCH
		SAVE IMAGE --push $IMAGE:$EARTHLY_GIT_BRANCH-alpine
	END

installer:
	FROM alpine

	COPY (+install/bluebuild --BUILD_TARGET="x86_64-unknown-linux-musl") /out/bluebuild
	COPY install.sh /install.sh

	CMD ["cat", "/install.sh"]

	ARG TAG
	ARG LATEST=false

	IF [ -n "$TAG" ]
		SAVE IMAGE --push $IMAGE:$TAG-installer

		IF [ "$LATEST" = "true" ]
			SAVE IMAGE --push $IMAGE:latest-installer
		END
	ELSE
		ARG EARTHLY_GIT_BRANCH
		SAVE IMAGE --push $IMAGE:$EARTHLY_GIT_BRANCH-installer
	END

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign
