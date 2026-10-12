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

COPY verify/smoke.sh /usr/local/bin/reference-smoke
RUN chmod 0755 /usr/local/bin/reference-smoke \
    && ldconfig \
    && reference-smoke

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/reference-smoke"]
