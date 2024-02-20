# This stage is responsible for holding onto
# your config without copying it directly into
# the final image
FROM scratch as stage-config
COPY ./config /config

# Copy modules
# The default modules are inside ublue-os/bling
# Custom modules overwrite defaults
FROM scratch as stage-modules
COPY --from=ghcr.io/ublue-os/bling:latest /modules /modules
COPY ./modules /modules

# This stage is responsible for holding onto
# exports like the exports.sh
FROM docker.io/alpine as stage-exports
RUN printf "#!/usr/bin/env bash\n\nget_yaml_array() { \n  readarray -t \"\$1\" < <(echo \"\$3\" | yq -I=0 \"\$2\")\n} \n\nexport -f get_yaml_array\nexport OS_VERSION=\$(grep -Po '(?<=VERSION_ID=)\d+' /usr/lib/os-release)" >> /exports.sh && chmod +x /exports.sh

FROM ghcr.io/ublue-os/silverblue-main:39

LABEL org.blue-build.build-id="f83a7190-d946-462d-8934-d69b844136c4"
LABEL org.opencontainers.image.title="template"
LABEL org.opencontainers.image.description="This is my personal OS image."
LABEL io.artifacthub.package.readme-url=https://raw.githubusercontent.com/blue-build/cli/main/README.md

ARG RECIPE=./config/recipe.yml
ARG IMAGE_REGISTRY=localhost

COPY --from=docker.io/mikefarah/yq /usr/bin/yq /usr/bin/yq
COPY --from=gcr.io/projectsigstore/cosign /ko-app/cosign /usr/bin/cosign

COPY --from=ghcr.io/blue-build/cli:latest-installer /out/bluebuild /usr/bin/bluebuild

ARG CONFIG_DIRECTORY="/tmp/config"
ARG IMAGE_NAME="template"
ARG BASE_IMAGE="ghcr.io/ublue-os/silverblue-main"
COPY ./config/files/usr /usr
RUN \
  --mount=type=tmpfs,target=/tmp \
  --mount=type=tmpfs,target=/var \
  --mount=type=bind,from=stage-config,src=/config,dst=/tmp/config,rw \
--mount=type=bind,from=stage-modules,src=/modules,dst=/tmp/modules,rw \
--mount=type=bind,from=stage-exports,src=/exports.sh,dst=/tmp/exports.sh \
  --mount=type=cache,dst=/var/cache/rpm-ostree,id=rpm-ostree-cache-template-39,sharing=locked \
  chmod +x /tmp/modules/script/script.sh \
  && source /tmp/exports.sh && /tmp/modules/script/script.sh '{"type":"script","scripts":["example.sh"]}'
RUN \
  --mount=type=tmpfs,target=/tmp \
  --mount=type=tmpfs,target=/var \
  --mount=type=bind,from=stage-config,src=/config,dst=/tmp/config,rw \
--mount=type=bind,from=stage-modules,src=/modules,dst=/tmp/modules,rw \
--mount=type=bind,from=stage-exports,src=/exports.sh,dst=/tmp/exports.sh \
  --mount=type=cache,dst=/var/cache/rpm-ostree,id=rpm-ostree-cache-template-39,sharing=locked \
  chmod +x /tmp/modules/rpm-ostree/rpm-ostree.sh \
  && source /tmp/exports.sh && /tmp/modules/rpm-ostree/rpm-ostree.sh '{"type":"rpm-ostree","repos":["https://copr.fedorainfracloud.org/coprs/atim/starship/repo/fedora-%OS_VERSION%/atim-starship-fedora-%OS_VERSION%.repo"],"install":["micro","starship"],"remove":["firefox","firefox-langpacks"]}'
RUN \
  --mount=type=tmpfs,target=/tmp \
  --mount=type=tmpfs,target=/var \
  --mount=type=bind,from=stage-config,src=/config,dst=/tmp/config,rw \
--mount=type=bind,from=stage-modules,src=/modules,dst=/tmp/modules,rw \
--mount=type=bind,from=stage-exports,src=/exports.sh,dst=/tmp/exports.sh \
  --mount=type=cache,dst=/var/cache/rpm-ostree,id=rpm-ostree-cache-template-39,sharing=locked \
  chmod +x /tmp/modules/default-flatpaks/default-flatpaks.sh \
  && source /tmp/exports.sh && /tmp/modules/default-flatpaks/default-flatpaks.sh '{"type":"default-flatpaks","notify":true,"system":{"install":["org.mozilla.firefox","org.gnome.Loupe","one.ablaze.floorp//lightning"],"remove":["org.gnome.eog"]}}'
RUN \
  --mount=type=tmpfs,target=/tmp \
  --mount=type=tmpfs,target=/var \
  --mount=type=bind,from=stage-config,src=/config,dst=/tmp/config,rw \
--mount=type=bind,from=stage-modules,src=/modules,dst=/tmp/modules,rw \
--mount=type=bind,from=stage-exports,src=/exports.sh,dst=/tmp/exports.sh \
  --mount=type=cache,dst=/var/cache/rpm-ostree,id=rpm-ostree-cache-template-39,sharing=locked \
  chmod +x /tmp/modules/signing/signing.sh \
  && source /tmp/exports.sh && /tmp/modules/signing/signing.sh '{"type":"signing"}'
RUN \
  --mount=type=tmpfs,target=/tmp \
  --mount=type=tmpfs,target=/var \
  --mount=type=bind,from=stage-config,src=/config,dst=/tmp/config,rw \
--mount=type=bind,from=stage-modules,src=/modules,dst=/tmp/modules,rw \
--mount=type=bind,from=stage-exports,src=/exports.sh,dst=/tmp/exports.sh \
  --mount=type=cache,dst=/var/cache/rpm-ostree,id=rpm-ostree-cache-template-39,sharing=locked \
  chmod +x /tmp/modules/test-module/test-module.sh \
  && source /tmp/exports.sh && /tmp/modules/test-module/test-module.sh '{"type":"test-module"}'

RUN ostree container commit