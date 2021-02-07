cmake_minimum_required(VERSION 3.14 FATAL_ERROR)
project(oiio-builder)

include(FetchContent)
set(BUILD_SHARED_LIBS OFF CACHE INTERNAL "")


# --- Boost ------------------------------------
FetchContent_Declare(
  boost
  GIT_REPOSITORY https://github.com/boostorg/boost.git
  GIT_TAG        boost-1.75.0
)
FetchContent_GetProperties(boost)
if(NOT boost_POPULATED)
  FetchContent_Populate(boost)
  add_subdirectory(${boost_SOURCE_DIR} ${boost_BINARY_DIR})
endif()



# --- OpenEXR ------------------------------------
FetchContent_Declare(
  openexr
  GIT_REPOSITORY https://github.com/AcademySoftwareFoundation/openexr.git
  GIT_TAG        v2.5.4
)
FetchContent_GetProperties(openexr)
if(NOT openexr_POPULATED)
  FetchContent_Populate(openexr)
  add_subdirectory(${openexr_SOURCE_DIR} ${openexr_BINARY_DIR})
endif()



# --- libtiff ------------------------------------
FetchContent_Declare(
  libtiff
  GIT_REPOSITORY https://gitlab.com/libtiff/libtiff.git
  GIT_TAG        v4.2.0
)
FetchContent_GetProperties(libtiff)
if(NOT libtiff_POPULATED)
  FetchContent_Populate(libtiff)
  add_subdirectory(${libtiff_SOURCE_DIR} ${libtiff_BINARY_DIR})
endif()



# --- libraw ------------------------------------
FetchContent_Declare(
  libraw
  GIT_REPOSITORY https://github.com/LibRaw/LibRaw.git
  GIT_TAG        0.20.2
)
FetchContent_GetProperties(libraw)
if(NOT libraw_POPULATED)
  FetchContent_Populate(libraw)
  add_subdirectory(${libraw_SOURCE_DIR} ${libraw_BINARY_DIR})
endif()



# --- libheif ------------------------------------
FetchContent_Declare(
  libheif
  GIT_REPOSITORY https://github.com/strukturag/libheif.git
  GIT_TAG        v1.11.0
)
FetchContent_GetProperties(libheif)
if(NOT libheif_POPULATED)
  FetchContent_Populate(libheif)
  add_subdirectory(${libheif_SOURCE_DIR} ${libheif_BINARY_DIR})
endif()



# --- OCIO ------------------------------------
FetchContent_Declare(
  opencolorio
  GIT_REPOSITORY https://github.com/AcademySoftwareFoundation/OpenColorIO
  GIT_TAG        v2.0.0
)
FetchContent_GetProperties(opencolorio)
if(NOT opencolorio_POPULATED)
  FetchContent_Populate(opencolorio)
  add_subdirectory(${opencolorio_SOURCE_DIR} ${opencolorio_BINARY_DIR})
endif()



# --- OpenImageIO ------------------------------------
FetchContent_Declare(
  openimageio
  GIT_REPOSITORY https://github.com/OpenImageIO/oiio.git
  GIT_TAG        v2.2.11.1
)
FetchContent_GetProperties(openimageio)
if(NOT openimageio_POPULATED)
  FetchContent_Populate(openimageio)
  add_subdirectory(${openimageio_SOURCE_DIR} ${openimageio_BINARY_DIR})
endif()