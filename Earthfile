VERSION 0.7

ARG FEDORA_MAJOR_VERSION=38

FROM registry.fedoraproject.org/fedora-toolbox:${FEDORA_MAJOR_VERSION}

ARG --global IMAGE=registry.gitlab.com/wunker-bunker/ublue-cli

iso-generator:
    GIT CLONE https://github.com/ublue-os/isogenerator.git /isogenerator
    WORKDIR /isogenerator
    ARG PACKAGES=$(cat deps.txt)
    RUN dnf install --disablerepo="*" --enablerepo="fedora,updates" --setopt install_weak_deps=0 --assumeyes $PACKAGES

    SAVE IMAGE --push $IMAGE/iso-generator

install:
	FROM rust
	COPY . /app
	WORKDIR /app

	RUN cargo build --release

	SAVE ARTIFACT target/release/ublue

ublue-cli:
	BUILD +install

	COPY +install/ublue /usr/bin/ublue

	ARG TAG
	IF [ "$TAG" != "" ]
	    SAVE IMAGE --push $IMAGE:$TAG

		ARG LATEST=false

		IF [ "$LATEST" = "true" ]
		    SAVE IMAGE --push $IMAGE:latest
		END
	END

all:
	BUILD +ublue-cli
	BUILD +iso-generator
