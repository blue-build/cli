VERSION 0.8
PROJECT blue-build/cli

IMPORT github.com/blue-build/earthly-lib/rust AS rust
# IMPORT ../earthly-lib/rust AS rust

ARG --global IMAGE=ghcr.io/blue-build/cli
ARG --global TAGGED="false"
ARG --global LATEST="false"

all:
    WAIT
        BUILD --platform=linux/amd64 --platform=linux/arm64 +prebuild
    END
    BUILD +build
    BUILD ./integration-tests+all

run-checks:
    BUILD +lint
    BUILD +test

build-images-all:
    BUILD --platform=linux/amd64 --platform=linux/arm64 +build-images

build-scripts-all:
    BUILD --platform=linux/amd64 --platform=linux/arm64 +build-scripts

build-images:
    BUILD +blue-build-cli
    BUILD +blue-build-cli-distrobox
    BUILD +installer

prebuild:
    BUILD +blue-build-cli-prebuild
    BUILD +blue-build-cli-distrobox-prebuild

lint:
    FROM +common
    RUN cargo fmt --all --check
    DO rust+CARGO --args="clippy --workspace"
    DO rust+CARGO --args="clippy --workspace --all-features"
    DO rust+CARGO --args="clippy --workspace --no-default-features"
    DO +EACH_PACKAGE --args="clippy --workspace --no-default-features"

test:
    FROM +common
    COPY --dir test-files/ integration-tests/ /app
    COPY +cosign/cosign /usr/bin/cosign

    DO rust+CARGO --args="test --workspace"
    DO rust+CARGO --args="test --workspace --all-features"
    DO rust+CARGO --args="test --workspace --no-default-features"
    DO +EACH_PACKAGE --args="test --no-default-features"

EACH_PACKAGE:
    FUNCTION
    ARG packages="$(cargo metadata --format-version 1 | jq -cr '.workspace_members | .[]' | sed 's|.*#||' | sed 's|@.*||')"

    ARG --required args

    FOR package IN $packages
        DO +EACH_FEAT --package="$package" --args="$args"
    END

EACH_FEAT:
    FUNCTION
    ARG --required package
    ARG features="$(cargo metadata --format-version 1 | jq -cr ".packages[] | select(.name == \"$package\") | .features | keys | .[] | select(. != \"default\")")"

    ARG --required args

    FOR feat IN $features
        DO rust+CARGO --args="$args --package $package --features $feat"
    END

install:
    FROM +common
    ARG --required BUILD_TARGET
    ARG --required RELEASE

    IF [ "$RELEASE" = "true" ]
        DO rust+CROSS --target="$BUILD_TARGET" --output="$BUILD_TARGET/release/[^\./]+"
        SAVE ARTIFACT target/$BUILD_TARGET/release/bluebuild
    ELSE
        DO rust+CROSS --args="build" --target="$BUILD_TARGET" --output="$BUILD_TARGET/debug/[^\./]+"
        SAVE ARTIFACT target/$BUILD_TARGET/debug/bluebuild
    END

install-all-features:
    FROM +common
    ARG --required BUILD_TARGET
    ARG --required RELEASE

    IF [ "$RELEASE" = "true" ]
        DO rust+CROSS --args="build --all-features --release" --target="$BUILD_TARGET" --output="$BUILD_TARGET/release/[^\./]+"
        SAVE ARTIFACT target/$BUILD_TARGET/release/bluebuild
    ELSE
        DO rust+CROSS --args="build --all-features" --target="$BUILD_TARGET" --output="$BUILD_TARGET/debug/[^\./]+"
        SAVE ARTIFACT target/$BUILD_TARGET/debug/bluebuild
    END

common:
    FROM --platform=native ghcr.io/blue-build/earthly-lib/cargo-builder

    RUN rustup self update && \
        rustup toolchain add stable && \
        rustup default stable && \
        rustup component add clippy rustfmt && \
        rustup update

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

    DO --pass-args +SAVE_IMAGE --IMAGE="$IMAGE/build-scripts"

blue-build-cli-prebuild:
    ARG BASE_IMAGE="registry.fedoraproject.org/fedora-toolbox"
    FROM "$BASE_IMAGE"

    RUN dnf -y install dnf-plugins-core \
        && dnf config-manager addrepo \
            --from-repofile=https://download.docker.com/linux/fedora/docker-ce.repo \
        && dnf install --refresh -y docker-ce docker-ce-cli containerd.io \
            docker-buildx-plugin docker-compose-plugin \
            buildah podman skopeo dumb-init git

    ENTRYPOINT ["/usr/bin/dumb-init", "--"]

    COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
    LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
    LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

    COPY +cosign/cosign /usr/bin/cosign

    ARG EARTHLY_GIT_HASH
    ARG TARGETARCH
    SAVE IMAGE --push "$IMAGE:$EARTHLY_GIT_HASH-prebuild-$TARGETARCH"

blue-build-cli:
    FROM alpine
    ARG RELEASE="true"
    ARG TARGETARCH

    IF [ "$RELEASE" = "true" ]
        ARG EARTHLY_GIT_HASH
        FROM "$IMAGE:$EARTHLY_GIT_HASH-prebuild-$TARGETARCH"
    ELSE
        FROM +blue-build-cli-prebuild
    END

    IF [ "$TARGETARCH" = "arm64" ]
        DO +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-gnu" --RELEASE=$RELEASE
    ELSE
        DO +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-gnu" --RELEASE=$RELEASE
    END

    RUN mkdir -p /bluebuild
    WORKDIR /bluebuild
    CMD ["bluebuild"]

    DO --pass-args +SAVE_IMAGE

blue-build-cli-distrobox-prebuild:
    ARG BASE_IMAGE="alpine"
    FROM $BASE_IMAGE

    RUN apk update && apk add --no-cache \
        alpine-base git dumb-init buildah \
        podman skopeo bash-completion docs \
        gcompat libc-utils lsof man-pages \
        mandoc musl-utils openssh-client-default \
        pinentry tar vte3 which \
        bash bc bzip2 coreutils curl diffutils findmnt \
        findutils gnupg gpg iproute2 iputils keyutils \
        less libcap ncurses ncurses-terminfo net-tools \
        pigz rsync shadow sudo tcpdump tree tzdata unzip \
        util-linux util-linux-misc vulkan-loader wget \
        xauth xz zip procps

    ENTRYPOINT ["/usr/bin/dumb-init", "--"]

    COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
    LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
    LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

    COPY +cosign/cosign /usr/bin/cosign

    ARG EARTHLY_GIT_HASH
    ARG TARGETARCH
    SAVE IMAGE --push "$IMAGE:$EARTHLY_GIT_HASH-distrobox-prebuild-$TARGETARCH"

blue-build-cli-distrobox:
    ARG EARTHLY_GIT_HASH
    ARG TARGETARCH
    FROM "$IMAGE:$EARTHLY_GIT_HASH-distrobox-prebuild-$TARGETARCH"

    IF [ "$TARGETARCH" = "arm64" ]
        DO +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="aarch64-unknown-linux-musl"
    ELSE
        DO +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="x86_64-unknown-linux-musl"
    END

    DO --pass-args +SAVE_IMAGE --SUFFIX="-distrobox"

installer:
    ARG BASE_IMAGE="alpine"
    FROM $BASE_IMAGE

    COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest
    LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
    LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

    ARG TARGETARCH
    IF [ "$TARGETARCH" = "arm64" ]
        DO +INSTALL --OUT_DIR="/out/" --BUILD_TARGET="aarch64-unknown-linux-musl"
    ELSE
        DO +INSTALL --OUT_DIR="/out/" --BUILD_TARGET="x86_64-unknown-linux-musl"
    END

    COPY install.sh /install.sh

    CMD ["cat", "/install.sh"]

    DO --pass-args +SAVE_IMAGE --SUFFIX="-installer"
    SAVE ARTIFACT /out/bluebuild

cosign:
    FROM ghcr.io/sigstore/cosign/cosign:v2.5.0
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
    COPY --keep-ts --dir src/ template/ recipe/ utils/ process/ /app
    COPY --keep-ts Cargo.* /app

    RUN /bin/bash -c 'set -eo pipefail; cargo metadata --no-deps --format-version 1 \
    | jq -r ".packages[] | select(.name == \"blue-build\") .version" > /version'

    SAVE ARTIFACT /version

INSTALL:
    FUNCTION
    ARG --required BUILD_TARGET
    ARG --required OUT_DIR
    ARG RELEASE="true"

    IF [ "$TAGGED" = "true" ]
        COPY --platform=native (+install/bluebuild --BUILD_TARGET=$BUILD_TARGET --RELEASE=$RELEASE) $OUT_DIR
    ELSE
        COPY --platform=native (+install-all-features/bluebuild --BUILD_TARGET=$BUILD_TARGET --RELEASE=$RELEASE) $OUT_DIR
    END

SAVE_IMAGE:
    FUNCTION
    ARG SUFFIX=""
    ARG IMAGE="$IMAGE"

    COPY --platform=native +version/version /
    ARG VERSION="$(cat /version)"
    ARG MAJOR_VERSION="$(echo "$VERSION" | cut -d'.' -f1)"
    ARG MINOR_VERSION="$(echo "$VERSION" | cut -d'.' -f2)"
    ARG PATCH_VERSION="$(echo "$VERSION" | cut -d'.' -f3)"
    ARG BUILD_TIME="$(date -Iseconds)"
    DO --pass-args +LABELS

    IF [ "$TAGGED" = "true" ]
        SAVE IMAGE --push "${IMAGE}:v${VERSION}${SUFFIX}"

        IF [ "$LATEST" = "true" ]
            SAVE IMAGE --push "${IMAGE}:latest${SUFFIX}"
            SAVE IMAGE --push "${IMAGE}:v${MAJOR_VERSION}.${MINOR_VERSION}${SUFFIX}"
            SAVE IMAGE --push "${IMAGE}:v${MAJOR_VERSION}${SUFFIX}"
        END
    ELSE
        ARG EARTHLY_GIT_BRANCH
        ARG IMAGE_TAG="$(echo "${EARTHLY_GIT_BRANCH}" | sed 's|/|_|g')"
        SAVE IMAGE --push "${IMAGE}:${IMAGE_TAG}${SUFFIX}"
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

    IF [ "$TAGGED" = "true" ]
        ARG EARTHLY_GIT_BRANCH
        LABEL org.opencontainers.image.ref.name="$EARTHLY_GIT_BRANCH"
    ELSE
        LABEL org.opencontainers.image.ref.name="v$VERSION"
    END
