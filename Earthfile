VERSION 0.8
PROJECT blue-build/cli

IMPORT github.com/earthly/lib/rust AS rust

ARG --global IMAGE=ghcr.io/blue-build/cli

all:
	BUILD +build
	BUILD ./integration-tests+all

build:
	WAIT
		BUILD --platform=linux/amd64 --platform=linux/arm64 +build-scripts
	END
	BUILD +lint
	BUILD +test
	BUILD --platform=linux/amd64 --platform=linux/arm64 +blue-build-cli
	BUILD --platform=linux/amd64 --platform=linux/arm64 +blue-build-cli-alpine
	BUILD --platform=linux/amd64 --platform=linux/arm64 +installer

lint:
	FROM +common
	DO rust+CARGO --args="clippy -- -D warnings"
	DO rust+CARGO --args="clippy --all-features -- -D warnings"
	DO rust+CARGO --args="clippy --no-default-features -- -D warnings"

test:
	FROM +common
	DO rust+CARGO --args="test -- --show-output"
	DO rust+CARGO --args="test --all-features -- --show-output"
	DO rust+CARGO --args="test --no-default-features -- --show-output"

install:
	FROM +common
	ARG --required BUILD_TARGET

	DO rust+CROSS --target="$BUILD_TARGET" --output="$BUILD_TARGET/release/[^\./]+"

	SAVE ARTIFACT target/$BUILD_TARGET/release/bluebuild

install-all-features:
	FROM +common
	ARG --required BUILD_TARGET

	DO rust+CROSS --args="build --all-features --release" --target="$BUILD_TARGET" --output="$BUILD_TARGET/release/[^\./]+"

	SAVE ARTIFACT target/$BUILD_TARGET/release/bluebuild

common:
	FROM ghcr.io/blue-build/earthly-lib/cargo-builder

	WORKDIR /app
	COPY --keep-ts --dir src/ template/ recipe/ utils/ /app
	COPY --keep-ts Cargo.* /app
	COPY --keep-ts *.md /app
	COPY --keep-ts LICENSE /app
	COPY --keep-ts build.rs /app
	COPY --keep-ts --dir .git/ /app
	RUN touch build.rs

	DO rust+INIT --keep_fingerprints=true

build-scripts:
	FROM alpine
	LABEL org.opencontainers.image.source="https://github.com/blue-build/cli"
	COPY --dir scripts/ /
	FOR script IN "$(ls /scripts | grep -e '.*\.sh$')"
		RUN echo "Making ${script} executable" && \
			chmod +x "scripts/${script}"
	END

	DO --pass-args +SAVE_IMAGE --SUFFIX="-build-scripts"

blue-build-cli:
	ARG BASE_IMAGE="registry.fedoraproject.org/fedora-toolbox"
	FROM $BASE_IMAGE
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"

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

	LABEL org.opencontainers.image.base.digest="$(skopeo inspect "docker://$BASE_IMAGE" | jq -r '.Digest')"

	COPY +cosign/cosign /usr/bin/cosign

	ARG TARGETARCH
	IF [ "$TARGETARCH" = "arm64" ]
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-gnu"
	ELSE
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-gnu"
	END

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	ENTRYPOINT ["bluebuild"]

	DO --pass-args +SAVE_IMAGE

blue-build-cli-alpine:
	ARG BASE_IMAGE="alpine"
	FROM $BASE_IMAGE
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"

	RUN apk update && apk add buildah podman skopeo fuse-overlayfs jq

	LABEL org.opencontainers.image.base.digest="$(skopeo inspect "docker://$BASE_IMAGE" | jq -r '.Digest')"

	COPY +cosign/cosign /usr/bin/cosign

	ARG TARGETARCH
	IF [ "$TARGETARCH" = "arm64" ]
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-musl"
	ELSE
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-musl"
	END

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	ENTRYPOINT ["bluebuild"]

	DO --pass-args +SAVE_IMAGE --SUFFIX="-alpine"

installer:
	ARG BASE_IMAGE="alpine"
	FROM $BASE_IMAGE
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"

	RUN apk update && apk add skopeo jq

	LABEL org.opencontainers.image.base.digest="$(skopeo inspect "docker://$BASE_IMAGE" | jq -r '.Digest')"

	ARG TARGETARCH
	IF [ "$TARGETARCH" = "arm64" ]
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-musl"
	ELSE
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-musl"
	END

	COPY install.sh /install.sh

	CMD ["cat", "/install.sh"]

	DO --pass-args +SAVE_IMAGE --SUFFIX="-installer"
	SAVE ARTIFACT /out/bluebuild

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign

version:
	FROM rust

	RUN apt-get update && apt-get install -y jq

	WORKDIR /app
	COPY --keep-ts --dir src/ template/ recipe/ utils/ /app
	COPY --keep-ts Cargo.* /app

	RUN echo "$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "blue-build") .version')" > /version

	SAVE ARTIFACT /version

INSTALL:
	FUNCTION
	ARG TAGGED="false"
	ARG --required BUILD_TARGET
	ARG --required OUT_DIR

	IF [ "$TAGGED" = "true" ]
		COPY (+install/bluebuild --BUILD_TARGET="$BUILD_TARGET") $OUT_DIR
	ELSE
		COPY (+install-all-features/bluebuild --BUILD_TARGET="$BUILD_TARGET") $OUT_DIR
	END

SAVE_IMAGE:
	FUNCTION
	ARG SUFFIX=""
	ARG TAGGED="false"

	COPY --platform=native +version/version /
	ARG VERSION="$(cat /version)"
	ARG MAJOR_VERSION="$(echo "$VERSION" | cut -d'.' -f1)"
	ARG MINOR_VERSION="$(echo "$VERSION" | cut -d'.' -f2)"
	ARG PATCH_VERSION="$(echo "$VERSION" | cut -d'.' -f3)"
	ARG BUILD_TIME="$(date -Iseconds)"
	DO --pass-args +LABELS

	IF [ "$TAGGED" = "true" ]
		SAVE IMAGE --push "${IMAGE}:v${VERSION}${SUFFIX}"

		ARG LATEST=false
		IF [ "$LATEST" = "true" ]
			SAVE IMAGE --push "${IMAGE}:latest${SUFFIX}"
			SAVE IMAGE --push "${IMAGE}:v${MAJOR_VERSION}.${MINOR_VERSION}${SUFFIX}"
			SAVE IMAGE --push "${IMAGE}:v${MAJOR_VERSION}${SUFFIX}"
		END
	ELSE
		ARG EARTHLY_GIT_BRANCH
		SAVE IMAGE --push "${IMAGE}:${EARTHLY_GIT_BRANCH}${SUFFIX}"
	END
	ARG EARTHLY_GIT_HASH
	SAVE IMAGE --push "${IMAGE}:${EARTHLY_GIT_HASH}${SUFFIX}"

LABELS:
	FUNCTION
	LABEL org.opencontainers.image.created="$BUILD_TIME"
	LABEL org.opencontainers.image.url="https://github.com/blue-build/cli"
	LABEL org.opencontainers.image.source="https://github.com/blue-build/cli"
	LABEL org.opencontainers.image.version="$VERSION"
	LABEL version="$VERSION"
	LABEL org.opencontainers.image.vendor="BlueBuild"
	LABEL vendor="BlueBuild"
	LABEL org.opencontainers.image.licenses="Apache-2.0"
	LABEL license="Apache-2.0"
	LABEL org.opencontainers.image.title="BlueBuild CLI tool"
	LABEL name="blue-build/cli"
	LABEL org.opencontainers.image.description="A CLI tool built for creating Containerfile templates for ostree based atomic distros"
	LABEL org.opencontainers.image.documentation="https://raw.githubusercontent.com/blue-build/cli/main/README.md"

	ARG TAGGED="false"
	IF [ "$TAGGED" = "true" ]
		ARG EARTHLY_GIT_BRANCH
		LABEL org.opencontainers.image.ref.name="$EARTHLY_GIT_BRANCH"
	ELSE
		LABEL org.opencontainers.image.ref.name="v$VERSION"
	END
