VERSION 0.8
PROJECT blue-build/cli

IMPORT github.com/blue-build/earthly-lib/rust AS rust
# IMPORT ../earthly-lib/rust AS rust

FROM alpine
ARG --global IMAGE=ghcr.io/blue-build/cli
ARG --global TAGGED="false"
ARG --global LATEST="false"

all:
    WAIT
        BUILD --platform=linux/amd64 --platform=linux/arm64 +prebuild
    END
    BUILD +build-images-all
    BUILD ./integration-tests+all

run-checks:
    BUILD +lint
    BUILD +test

build-images-all:
    WAIT
        BUILD --platform=linux/amd64 --platform=linux/arm64 +build-images
    END

    ARG EARTHLY_PUSH
    IF [ "$EARTHLY_PUSH" = "true" ]
        BUILD --pass-args +sign-all
    END

sign-all:
    ARG SUFFIX_LIST="- distrobox installer"
    BUILD --pass-args +sign-images
    COPY --pass-args +digest-list/digest-list /
    SAVE ARTIFACT /digest-list AS LOCAL ./digest-list

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

    ENV RUSTUP_PERMIT_COPY_RENAME="true"
    RUN rustup self update && \
        rustup toolchain add stable && \
        rustup default stable && \
        rustup component add clippy rustfmt && \
        rustup update

    WORKDIR /app
    COPY --keep-ts --dir \
        build.rs \
        LICENSE \
        *.md \
        Cargo.* \
        src/ \
        template/ \
        recipe/ \
        utils/ \
        process/ \
        scripts/ \
        .git/ \
        /app

    DO rust+INIT --keep_fingerprints=true

blue-build-cli-prebuild:
    ARG BASE_IMAGE="registry.fedoraproject.org/fedora-toolbox:42"
    FROM "$BASE_IMAGE"

    RUN dnf5 -y update \
        && dnf5 -y reinstall shadow-utils \
        && dnf5 -y install dnf5-plugins \
        && dnf5 config-manager addrepo \
            --from-repofile=https://download.docker.com/linux/fedora/docker-ce.repo \
        && dnf5 -y install --refresh \
            docker-ce docker-ce-cli containerd.io \
            docker-buildx-plugin docker-compose-plugin \
            buildah podman skopeo dumb-init git fuse-overlayfs \
            containers-common rpm-ostree bootc \
        && rm -rf /var/cache /var/log/dnf* /var/log/yum.*

    COPY image_files/containers.conf /etc/containers/
    COPY image_files/entrypoint.sh /entrypoint.sh

    RUN chmod 644 /etc/containers/containers.conf \
        && chmod +x /entrypoint.sh

    RUN sed -e 's|^#mount_program|mount_program|g' \
        -e '/additionalimage.*/a "/var/lib/shared",' \
        -e 's|^mountopt[[:space:]]*=.*$|mountopt = "nodev,fsync=0"|g' \
        /usr/share/containers/storage.conf \
        > /etc/containers/storage.conf

    # Setup internal Podman to pass subscriptions down from host to internal container
    RUN printf '/run/secrets/etc-pki-entitlement:/run/secrets/etc-pki-entitlement\n/run/secrets/rhsm:/run/secrets/rhsm\n' \
        > /etc/containers/mounts.conf

    COPY +cosign/cosign /usr/bin/cosign
    COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$BASE_IMAGE) /base-image-digest

    LABEL org.opencontainers.image.base.name="$BASE_IMAGE"
    LABEL org.opencontainers.image.base.digest="$(cat /base-image-digest)"

    VOLUME /var/lib/containers

    RUN mkdir -p \
            /var/lib/shared/overlay-images \
            /var/lib/shared/overlay-layers \
            /var/lib/shared/vfs-images \
            /var/lib/shared/vfs-layers \
        && touch /var/lib/shared/overlay-images/images.lock \
        && touch /var/lib/shared/overlay-layers/layers.lock \
        && touch /var/lib/shared/vfs-images/images.lock \
        && touch /var/lib/shared/vfs-layers/layers.lock

    ENV _CONTAINERS_USERNS_CONFIGURED=""
    ENV BUILDAH_ISOLATION="chroot"

    ENTRYPOINT ["/entrypoint.sh"]

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

    DO +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="$(uname -m)-unknown-linux-gnu" --RELEASE=$RELEASE

    WORKDIR /bluebuild

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

    DO +INSTALL --OUT_DIR="/usr/bin/" --BUILD_TARGET="$(uname -m)-unknown-linux-musl"

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
    FROM ghcr.io/sigstore/cosign/cosign:v3.0.3
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

digest-list:
    FROM alpine

    COPY --platform=native +version/version /
    LET version="$(cat /version)"
    LET major_version="$(echo "$version" | cut -d'.' -f1)"
    LET minor_version="$(echo "$version" | cut -d'.' -f2)"

    ARG --required SUFFIX_LIST
    ARG EARTHLY_GIT_HASH
    ARG EARTHLY_GIT_BRANCH
    LET suffix=""

    FOR s IN $SUFFIX_LIST
        # The '-' character will be used for no suffix
        IF [ "$s" = "-" ]
            SET suffix=""
        ELSE
            SET suffix="-$s"
        END

        IF [ "$TAGGED" = "true" ]
            DO +PRINT_IMAGE_DIGEST --IMAGE="${IMAGE}:v${version}${suffix}"
            IF [ "$LATEST" = "true" ]
                DO +PRINT_IMAGE_DIGEST --IMAGE="${IMAGE}:latest${suffix}"
                DO +PRINT_IMAGE_DIGEST --IMAGE="${IMAGE}:v${major_version}.${minor_version}${suffix}"
                DO +PRINT_IMAGE_DIGEST --IMAGE="${IMAGE}:v${major_version}${suffix}"
            END
        ELSE
            DO +PRINT_IMAGE_DIGEST --IMAGE="${IMAGE}:$(echo "${EARTHLY_GIT_BRANCH}" | sed 's|/|_|g')${suffix}"
        END
        DO +PRINT_IMAGE_DIGEST --IMAGE="${IMAGE}:${EARTHLY_GIT_HASH}${suffix}"
    END

    SAVE ARTIFACT /digest-list

sign-images:
    FROM alpine
    COPY +cosign/cosign /usr/bin/

    ARG --required SUFFIX_LIST
    COPY --pass-args +digest-list/digest-list /
    COPY cosign.pub /

    ENV COSIGN_YES="true"
    ENV COSIGN_PASSWORD=""
    RUN --push \
        --secret GH_TOKEN \
        --secret GH_ACTOR \
        echo "$GH_TOKEN" | cosign login ghcr.io --username "$GH_ACTOR" --password-stdin

    FOR digest IN $(cat /digest-list | sed -E "s|^${IMAGE}:[^,]+,(sha256:[a-f0-9]+)$|\1|g" | sort -u)
        RUN --push --secret COSIGN_PRIVATE_KEY \
            cosign sign \
                --new-bundle-format=false \
                --use-signing-config=false \
                --key=env://COSIGN_PRIVATE_KEY \
                --recursive \
                "${IMAGE}@${digest}"
        RUN --push cosign verify --key=/cosign.pub "${IMAGE}@${digest}"
    END

PRINT_IMAGE_DIGEST:
    FUNCTION
    ARG --required IMAGE
    COPY --platform=native (+digest/base-image-digest --BASE_IMAGE=$IMAGE) /base-image-digest
    RUN echo "${IMAGE},$(cat /base-image-digest)" >> /digest-list

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
    LET version="$(cat /version)"
    LET major_version="$(echo "$version" | cut -d'.' -f1)"
    LET minor_version="$(echo "$version" | cut -d'.' -f2)"
    DO --pass-args +LABELS

    IF [ "$TAGGED" = "true" ]
        SAVE IMAGE --push "${IMAGE}:v${version}${SUFFIX}"

        IF [ "$LATEST" = "true" ]
            SAVE IMAGE --push "${IMAGE}:latest${SUFFIX}"
            SAVE IMAGE --push "${IMAGE}:v${major_version}.${minor_version}${SUFFIX}"
            SAVE IMAGE --push "${IMAGE}:v${major_version}${SUFFIX}"
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
    LET build_time="$(date -Iseconds)"
    LABEL org.opencontainers.image.created="$build_time"
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
