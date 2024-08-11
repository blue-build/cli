VERSION 0.8
PROJECT blue-build/cli

IMPORT github.com/earthly/lib/rust AS rust

ARG --global IMAGE=ghcr.io/blue-build/cli

all:
	WAIT
		BUILD --platform=linux/amd64 --platform=linux/arm64 +prebuild
	END
	BUILD +build
	BUILD ./integration-tests+all

build:
	WAIT
		BUILD --platform=linux/amd64 --platform=linux/arm64 +build-scripts
	END
	BUILD --platform=linux/amd64 --platform=linux/arm64 +build-images

run-checks:
	BUILD +lint
	BUILD +test

build-images:
	BUILD +blue-build-cli
	BUILD +blue-build-cli-alpine
	BUILD +installer

prebuild:
	BUILD +blue-build-cli-prebuild
	BUILD +blue-build-cli-alpine-prebuild

lint:
	FROM +common
	RUN cargo fmt --check
	DO rust+CARGO --args="clippy -- -D warnings"
	DO rust+CARGO --args="clippy --all-features -- -D warnings"
	DO rust+CARGO --args="clippy --no-default-features -- -D warnings"

test:
	FROM +common
	COPY --dir test-files/ integration-tests/ /app
	COPY +cosign/cosign /usr/bin/cosign

	DO rust+CARGO --args="test --workspace -- --show-output"
	DO rust+CARGO --args="test --workspace --all-features -- --show-output"
	DO rust+CARGO --args="test --workspace --no-default-features -- --show-output"

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
	FROM --platform=native ghcr.io/blue-build/earthly-lib/cargo-builder

	WORKDIR /app
	COPY --keep-ts --dir src/ template/ recipe/ utils/ process/ /app
	COPY --keep-ts Cargo.* /app
	COPY --keep-ts *.md /app
	COPY --keep-ts LICENSE /app
	COPY --keep-ts build.rs /app
	COPY --keep-ts --dir .git/ /app
	RUN touch build.rs

	DO rust+INIT --keep_fingerprints=true

build-scripts:
	ARG BASE_IMAGE="alpine"
	FROM $BASE_IMAGE

	COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
	LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

	COPY --dir scripts/ /
	FOR script IN "$(ls /scripts | grep -e '.*\.sh$')"
		RUN echo "Making ${script} executable" && \
			chmod +x "scripts/${script}"
	END

	DO --pass-args +SAVE_IMAGE --SUFFIX="-build-scripts"

blue-build-cli-prebuild:
	ARG BASE_IMAGE="registry.fedoraproject.org/fedora-toolbox"
	FROM DOCKERFILE -f Dockerfile.fedora .

	COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
	LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

	COPY +cosign/cosign /usr/bin/cosign
	ARG EARTHLY_GIT_HASH
	ARG TARGETARCH
	SAVE IMAGE --push "$IMAGE:$EARTHLY_GIT_HASH-prebuild-$TARGETARCH"

blue-build-cli:
	ARG EARTHLY_GIT_HASH
	ARG TARGETARCH
	FROM "$IMAGE:$EARTHLY_GIT_HASH-prebuild-$TARGETARCH"

	IF [ "$TARGETARCH" = "arm64" ]
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-gnu"
	ELSE
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-gnu"
	END

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	CMD ["bluebuild"]

	DO --pass-args +SAVE_IMAGE

blue-build-cli-alpine-prebuild:
	ARG BASE_IMAGE="alpine"
	FROM DOCKERFILE -f Dockerfile.alpine .

	COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
	LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

	COPY +cosign/cosign /usr/bin/cosign

	ARG EARTHLY_GIT_HASH
	ARG TARGETARCH
	SAVE IMAGE --push "$IMAGE:$EARTHLY_GIT_HASH-alpine-prebuild-$TARGETARCH"

blue-build-cli-alpine:
	ARG EARTHLY_GIT_HASH
	ARG TARGETARCH
	FROM "$IMAGE:$EARTHLY_GIT_HASH-alpine-prebuild-$TARGETARCH"

	IF [ "$TARGETARCH" = "arm64" ]
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-musl"
	ELSE
		DO --pass-args +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-musl"
	END

	RUN mkdir -p /bluebuild
	WORKDIR /bluebuild
	CMD ["bluebuild"]

	DO --pass-args +SAVE_IMAGE --SUFFIX="-alpine"

installer:
	ARG BASE_IMAGE="alpine"
	FROM $BASE_IMAGE

	COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
	LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
	LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

	ARG TARGETARCH
	IF [ "$TARGETARCH" = "arm64" ]
		DO --pass-args +INSTALL --OUT_DIR="/out/" --BUILD_TARGET="aarch64-unknown-linux-musl"
	ELSE
		DO --pass-args +INSTALL --OUT_DIR="/out/" --BUILD_TARGET="x86_64-unknown-linux-musl"
	END

	COPY install.sh /install.sh

	CMD ["cat", "/install.sh"]

	DO --pass-args +SAVE_IMAGE --SUFFIX="-installer"
	SAVE ARTIFACT /out/bluebuild

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign

digest:
	FROM alpine
	RUN apk update && apk add skopeo jq

	ARG --required BASE_IMAGE
	RUN skopeo inspect "docker://$BASE_IMAGE" | jq -r '.Digest' > /base-image-digest
	SAVE ARTIFACT /base-image-digest
	
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
		COPY --platform=native (+install/bluebuild --BUILD_TARGET="$BUILD_TARGET") $OUT_DIR
	ELSE
		COPY --platform=native (+install-all-features/bluebuild --BUILD_TARGET="$BUILD_TARGET") $OUT_DIR
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
