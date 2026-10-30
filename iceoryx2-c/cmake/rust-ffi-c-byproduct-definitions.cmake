# Copyright (c) 2024 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache Software License 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
# which is available at https://opensource.org/licenses/MIT.
#
# SPDX-License-Identifier: Apache-2.0 OR MIT

set(ICEORYX2_C_INCLUDE_DIR ${RUST_BUILD_ARTIFACT_PATH}/iceoryx2-ffi-c-cbindgen/include)

set(ICEORYX2_C_LIB iceoryx2_ffi_c)
set(ICEORYX2_C_STATIC_LIB_LINK_NAME lib${ICEORYX2_C_LIB}.a)
if(WIN32 AND NOT MINGW)
    set(ICEORYX2_C_STATIC_LIB_LINK_NAME ${ICEORYX2_C_LIB}.lib)
endif()

set(ICEORYX2_C_STATIC_LIB_LINK_FILE ${RUST_BUILD_ARTIFACT_PATH}/${ICEORYX2_C_STATIC_LIB_LINK_NAME})
set(ICEORYX2_C_SHARED_LIB_LINK_NAME ${ICEORYX2_C_STATIC_LIB_LINK_NAME})
set(ICEORYX2_C_LIB_ARTIFACTS ${ICEORYX2_C_STATIC_LIB_LINK_FILE})
