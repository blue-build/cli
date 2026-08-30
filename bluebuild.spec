Name: bluebuild
Version: 0.9.6
Release: 1%{?dist}
Summary: BlueBuild's command line program that builds Containerfiles and custom images based on your recipe.yml.
License: Apache-2.0
URL: https://github.com/blue-build
Source0: https://github.com/blue-build/cli/archive/refs/tags/v%{version}.tar.gz
BuildRequires: cargo
Requires: podman,buildah

%description
%{summary}

%prep
tar -xf %{SOURCE0}
cd v%{version}/

%build
cargo build --release

%install
mkdir -p $RPM_BUILD_ROOT/usr/bin
install -Dm 755 target/release/bluebuild $RPM_BUILD_ROOT/usr/bin/bluebuild

%clean
rm -rf $RPM_BUILD_ROOT
