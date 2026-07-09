%global crate asciimeadow

Name:           asciimeadow
Version:        0.1.0
Release:        1%{?dist}
Summary:        Terminal ASCII meadow screensaver

# Le projet est sous MIT OR Apache-2.0 ; les crates embarquées (vendored)
# portent leurs propres licences permissives.
License:        MIT OR Apache-2.0
URL:            https://github.com/M4THIA5/asciimeadow
Source0:        %{url}/archive/v%{version}/%{crate}-%{version}.tar.gz
Source1:        %{crate}-%{version}-vendor.tar.xz

BuildRequires:  cargo-rpm-macros >= 24

%description
Économiseur d'écran terminal : une prairie ASCII animée (arbre, animaux,
météo, cycle jour/nuit) rendue avec crossterm.

%prep
%autosetup -n %{crate}-%{version} -a1
%cargo_prep -v vendor

%build
%cargo_build

%install
install -Dpm 0755 -t %{buildroot}%{_bindir} $(find target -type f -name %{crate} -perm -u+x)

%check
%cargo_test

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md
%{_bindir}/asciimeadow

%changelog
* Wed Jul 08 2026 Mathias Collas <mathias.collas@gmail.com> - 0.1.0-1
- Paquet initial
