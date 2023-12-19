VERSION 0.7

FROM registry.fedoraproject.org/fedora-toolbox

ARG --global IMAGE

cosign:
    FROM gcr.io/projectsigstore/cosign 
    
    SAVE ARTIFACT /ko-app/cosign cosign

install:
	FROM rust
	COPY . /app
	WORKDIR /app

	RUN cargo build --release

	SAVE ARTIFACT target/release/ublue

build:
	BUILD +install

    RUN dnf install --refresh -y podman buildah skopeo

    COPY +cosign/cosign /usr/bin/cosign
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
	BUILD --platform=linux/amd64 +build
