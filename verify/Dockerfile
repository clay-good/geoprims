FROM ubuntu:24.04@sha256:534baea6a22c03a63003dbc8dbe78fe34bc0d7e595d9a9dc9834884ff530eb55

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        ca-certificates \
        cmake \
        curl \
        g++ \
        gcc \
        gfortran \
        libcurl4-openssl-dev \
        libssl-dev \
        libsqlite3-dev \
        libtiff-dev \
        ninja-build \
        nlohmann-json3-dev \
        sqlite3 \
        unzip \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /tmp/build

RUN curl -fL https://github.com/geographiclib/geographiclib/archive/refs/tags/v2.7.tar.gz -o geographiclib.tar.gz \
    && echo '78d77cc8216843b78d14b41970ed9de3dedc34b9cd50ad84af2849c63a41eff8  geographiclib.tar.gz' | sha256sum -c - \
    && tar -xzf geographiclib.tar.gz \
    && cmake -S geographiclib-2.7 -B geographiclib-build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DBUILD_SHARED_LIBS=OFF \
    && cmake --build geographiclib-build \
    && cmake --install geographiclib-build \
    && rm -rf geographiclib-2.7 geographiclib-build geographiclib.tar.gz

RUN curl -fL https://github.com/OSGeo/PROJ/archive/refs/tags/9.9.0.tar.gz -o proj.tar.gz \
    && echo '45a27f617f5665d29a6ea59332ec8beaf73bee35196800554385b09b336e2333  proj.tar.gz' | sha256sum -c - \
    && tar -xzf proj.tar.gz \
    && cmake -S PROJ-9.9.0 -B proj-build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DBUILD_TESTING=OFF \
    && cmake --build proj-build \
    && cmake --install proj-build \
    && rm -rf PROJ-9.9.0 proj-build proj.tar.gz

RUN curl -fL https://github.com/uber/h3/archive/refs/tags/v4.5.0.tar.gz -o h3.tar.gz \
    && echo '0da8a392a6ff77e76b60e6a331a49497d0935b6b7b6899da7a3e2786139b0441  h3.tar.gz' | sha256sum -c - \
    && tar -xzf h3.tar.gz \
    && cmake -S h3-4.5.0 -B h3-build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DBUILD_BENCHMARKS=OFF \
        -DBUILD_TESTING=OFF \
    && cmake --build h3-build \
    && cmake --install h3-build \
    && rm -rf h3-4.5.0 h3-build h3.tar.gz

RUN curl -fL https://github.com/abseil/abseil-cpp/archive/refs/tags/20250814.1.tar.gz -o abseil.tar.gz \
    && echo '1692f77d1739bacf3f94337188b78583cf09bab7e420d2dc6c5605a4f86785a1  abseil.tar.gz' | sha256sum -c - \
    && tar -xzf abseil.tar.gz \
    && cmake -S abseil-cpp-20250814.1 -B abseil-build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_CXX_STANDARD=17 \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DABSL_BUILD_TESTING=OFF \
        -DABSL_PROPAGATE_CXX_STD=ON \
    && cmake --build abseil-build \
    && cmake --install abseil-build \
    && rm -rf abseil-cpp-20250814.1 abseil-build abseil.tar.gz

RUN curl -fL https://github.com/google/s2geometry/archive/refs/tags/v0.13.1.tar.gz -o s2.tar.gz \
    && echo 'df001f8352dce083a87b74646bcbc65fbbcd039646bda5b64adfda1e2ea32d47  s2.tar.gz' | sha256sum -c - \
    && tar -xzf s2.tar.gz \
    && cmake -S s2geometry-0.13.1 -B s2-build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr/local \
        -DBUILD_EXAMPLES=OFF \
        -DBUILD_SHARED_LIBS=OFF \
        -DBUILD_TESTS=OFF \
    && cmake --build s2-build \
    && cmake --install s2-build \
    && rm -rf s2geometry-0.13.1 s2-build s2.tar.gz

COPY verify/s2-smoke/ /tmp/build/s2-smoke/
RUN cmake -S s2-smoke -B s2-smoke-build -G Ninja -DCMAKE_BUILD_TYPE=Release \
    && cmake --build s2-smoke-build \
    && install -m 0755 s2-smoke-build/s2-smoke /usr/local/bin/s2-smoke \
    && rm -rf s2-smoke s2-smoke-build

RUN curl -fL https://www.ngdc.noaa.gov/geomag/WMM/data/WMM2025/wmm2025_Windows.zip -o wmm.zip \
    && echo 'ca56bd64d8e4ed20e8994b3ea8da87867c49ddc029f6e18f332c2976061a9f40  wmm.zip' | sha256sum -c - \
    && unzip -q wmm.zip \
    && cc -O2 \
        -Iwmm2025_Windows/src \
        wmm2025_Windows/main/wmm_file.c \
        wmm2025_Windows/src/GeomagnetismLibrary.c \
        -lm -o /usr/local/bin/wmm2025-file \
    && install -d /usr/local/share/wmm2025 \
    && install -m 0644 wmm2025_Windows/bin/WMM.COF /usr/local/share/wmm2025/WMM.COF \
    && install -m 0644 wmm2025_Windows/bin/sample_coords.txt /usr/local/share/wmm2025/sample_coords.txt \
    && rm -rf wmm2025_Windows wmm.zip

RUN curl -fL https://geodesy.noaa.gov/TOOLS/Htdp/HTDP-download.zip -o htdp.zip \
    && echo 'cbc3bd587dd6132f57dee4d9d01e5ca171f86ddb96f8f11b9242260fbe691474  htdp.zip' | sha256sum -c - \
    && unzip -q htdp.zip -d htdp \
    && gfortran -O2 -std=legacy -ffixed-line-length-none \
        htdp/htdp.f htdp/initbd.f htdp/initeq.f htdp/initps.f htdp/initvl.f \
        -o /usr/local/bin/htdp \
    && rm -rf htdp htdp.zip

# The hydrology references (add-raster-hydrology task 2). GDAL is here for
# RichDEM's raster I/O, and its command-line tools make the grid the smoke
# cases run through both of them.
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
        gdal-bin \
        libgdal-dev \
    && rm -rf /var/lib/apt/lists/*

# RichDEM (GPLv3): built as command-line programs and used only as external
# processes, never linked into geoprims, which is what its licence requires
# of us.
RUN curl -fL https://github.com/r-barnes/richdem/archive/refs/tags/v2.3.1.tar.gz -o richdem.tar.gz \
    && echo 'b232484bcbbeec063de8b0d02bc33f616da4509e12a7137bc87ea0b838ddba83  richdem.tar.gz' | sha256sum -c - \
    && tar -xzf richdem.tar.gz \
    && cmake -S richdem-2.3.1 -B richdem-build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DUSE_GDAL=ON \
        -DRICHDEM_NO_PROGRESS=ON \
    && cmake --build richdem-build \
    && install -m 0755 \
        richdem-build/apps/rd_depressions_flood.exe \
        richdem-build/apps/rd_depressions_breach.exe \
        richdem-build/apps/rd_depressions_has.exe \
        richdem-build/apps/rd_d8_flowdirs.exe \
        richdem-build/apps/rd_flow_accumulation.exe \
        /usr/local/bin/ \
    && rm -rf richdem-2.3.1 richdem-build richdem.tar.gz

# WhiteboxTools Open Core (MIT). It publishes no release binaries, so this
# builds the workspace's command-line crate -- not the whitebox-runner GUI,
# which would pull in a window toolkit. rustup pins per target rather than as
# one installer, so the architecture is read rather than assumed, and the
# toolchain goes away with the sources.
#
# The lock file is removed rather than honoured: upstream's committed
# Cargo.lock carries a checksum for tsp-rs 0.1.0 that no longer matches what
# crates.io serves, and cargo refuses the build over it whether or not
# --locked is passed (CI runs 44957292545 and 44959074411). What is pinned
# here is the source archive's digest and, in the smoke case, the version the
# binary reports; resolving the dependency tree is cargo's job, and fixing
# that lock is upstream's.
RUN arch="$(dpkg --print-architecture)" \
    && case "$arch" in \
        amd64) target=x86_64-unknown-linux-gnu; \
               sum=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55 ;; \
        arm64) target=aarch64-unknown-linux-gnu; \
               sum=9f608c142e0e38a78957d0661ba1dfaf0b6fb37184cfaa4b9a0aa57af6184282 ;; \
        *) echo "no pinned rustup for $arch" >&2; exit 1 ;; \
    esac \
    && curl -fL "https://static.rust-lang.org/rustup/archive/1.29.3/$target/rustup-init" -o rustup-init \
    && echo "$sum  rustup-init" | sha256sum -c - \
    && chmod 0755 rustup-init \
    && ./rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.98.0 \
    && curl -fL https://github.com/jblindsay/whitebox-tools/archive/refs/tags/v2.4.0.tar.gz -o wbt.tar.gz \
    && echo 'f99f47e3d362cd3a1e8ac3f1e060ad7d0426244520ebd968f3ef01870fc1a955  wbt.tar.gz' | sha256sum -c - \
    && tar -xzf wbt.tar.gz \
    && rm -f whitebox-tools-2.4.0/Cargo.lock \
    && ( cd whitebox-tools-2.4.0 \
        && . "$HOME/.cargo/env" \
        && cargo build --release -p whitebox_tools ) \
    && install -m 0755 whitebox-tools-2.4.0/target/release/whitebox_tools /usr/local/bin/whitebox_tools \
    && ( . "$HOME/.cargo/env" && rustup self uninstall -y ) \
    && rm -rf whitebox-tools-2.4.0 wbt.tar.gz rustup-init "$HOME/.cargo" "$HOME/.rustup"

COPY verify/smoke.sh /usr/local/bin/reference-smoke
RUN chmod 0755 /usr/local/bin/reference-smoke \
    && ldconfig \
    && reference-smoke

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/reference-smoke"]
