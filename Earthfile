VERSION --global-cache 0.7
IMPORT github.com/earthly/lib/rust AS rust

ARG --global FEDORA_MAJOR_VERSION=38

ARG --global IMAGE=registry.gitlab.com/wunker-bunker/blue-build

iso-generator:
	FROM registry.fedoraproject.org/fedora-toolbox:${FEDORA_MAJOR_VERSION}

    GIT CLONE https://github.com/ublue-os/isogenerator.git /isogenerator
    WORKDIR /isogenerator
    ARG PACKAGES=$(cat deps.txt)
    RUN dnf install --disablerepo="*" --enablerepo="fedora,updates" --setopt install_weak_deps=0 --assumeyes $PACKAGES

    SAVE IMAGE --push $IMAGE/iso-generator

cosign:
	FROM gcr.io/projectsigstore/cosign
	SAVE ARTIFACT /ko-app/cosign

install:
	FROM rust
	DO rust+INIT --keep_fingerprints=true

	COPY --keep-ts . /app
	WORKDIR /app

	ARG --required TARGET
	DO rust+CARGO --args="build --release --target $TARGET" --output="$TARGET/release/[^\./]+"

	SAVE ARTIFACT target/$TARGET/release/bb

blue-build-cli:
	FROM registry.fedoraproject.org/fedora-toolbox:${FEDORA_MAJOR_VERSION}
	BUILD +install --TARGET="x86_64-unknown-linux-gnu"

	RUN dnf install --refresh -y buildah podman skopeo

	COPY +cosign/cosign /usr/bin/cosign
	COPY (+install/bb --TARGET="x86_64-unknown-linux-gnu") /usr/bin/bb

	ARG TAG
	IF [ "$TAG" != "" ]
	    SAVE IMAGE --push $IMAGE:$TAG

		ARG LATEST=false

		IF [ "$LATEST" = "true" ]
		    SAVE IMAGE --push $IMAGE:latest
		END
	ELSE
		SAVE IMAGE blue-build
	END

blue-build-cli-alpine:
	FROM alpine
	BUILD +install --TARGET="x86_64-unknown-linux-musl"

	RUN apk update && apk add buildah podman skopeo fuse-overlayfs

	COPY +cosign/cosign /usr/bin/cosign
	COPY (+install/bb --TARGET="x86_64-unknown-linux-musl") /usr/bin/bb

	ARG TAG
	IF [ "$TAG" != "" ]
	    SAVE IMAGE --push $IMAGE:$TAG-alpine

		ARG LATEST=false

		IF [ "$LATEST" = "true" ]
		    SAVE IMAGE --push $IMAGE:alpine
		END
	ELSE
		SAVE IMAGE blue-build:alpine
	END

all:
	BUILD +blue-build
	BUILD +blue-build-alpine
	BUILD +iso-generator
