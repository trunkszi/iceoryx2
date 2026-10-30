// Copyright (c) 2025 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#ifndef IOX2_SERVICE_BUILDER_INTERNAL_HPP
#define IOX2_SERVICE_BUILDER_INTERNAL_HPP

#include "iox2/bb/slice.hpp"
#include "iox2/bb/static_string.hpp"
#include "iox2/bb/static_vector.hpp"
#include "iox2/payload_info.hpp"
#include "iox2/type_name.hpp"

#include <cstdio>
#include <string_view>
#include <typeinfo>

namespace iox2 {
namespace internal {

template <typename>
auto get_type_name() -> TypeName;

template <typename Payload, typename = void>
struct HasPayloadTypeNameMember : std::false_type { };

template <typename Payload>
struct HasPayloadTypeNameMember<Payload, decltype((void) Payload::IOX2_TYPE_NAME)> : std::true_type { };

template <typename Payload, typename = void>
struct HasPayloadTypeNameSpecialization : std::false_type { };

template <typename Payload>
struct HasPayloadTypeNameSpecialization<Payload, decltype((void) TypeNameSpecialization<Payload>::value())>
    : std::true_type { };

template <typename Payload>
using FromPayloadTypeNameSpecialization = std::enable_if_t<HasPayloadTypeNameSpecialization<Payload>::value, TypeName>;

template <typename Payload>
using FromPayloadTypeNameMember =
    std::enable_if_t<!HasPayloadTypeNameSpecialization<Payload>::value && HasPayloadTypeNameMember<Payload>::value,
                     TypeName>;

template <typename Payload>
using FromNonSlice =
    std::enable_if_t<!HasPayloadTypeNameSpecialization<Payload>::value && !HasPayloadTypeNameMember<Payload>::value
                         && !bb::IsSlice<Payload>::VALUE && !iox2::bb::IsStaticVector<Payload>::value
                         && !iox2::bb::IsStaticString<Payload>::value,
                     TypeName>;

template <typename Payload>
using FromStaticVector =
    std::enable_if_t<!HasPayloadTypeNameSpecialization<Payload>::value && iox2::bb::IsStaticVector<Payload>::value,
                     TypeName>;

template <typename Payload>
using FromStaticString =
    std::enable_if_t<!HasPayloadTypeNameSpecialization<Payload>::value && iox2::bb::IsStaticString<Payload>::value,
                     TypeName>;

template <typename Payload>
using FromSliceWithInnerPayloadTypeNameMember =
    std::enable_if_t<!HasPayloadTypeNameSpecialization<Payload>::value && !HasPayloadTypeNameMember<Payload>::value
                         && bb::IsSlice<Payload>::VALUE && HasPayloadTypeNameMember<typename Payload::ValueType>::value,
                     TypeName>;

template <typename Payload>
using FromSliceWithoutInnerPayloadTypeNameMember =
    std::enable_if_t<!HasPayloadTypeNameSpecialization<Payload>::value && !HasPayloadTypeNameMember<Payload>::value
                         && bb::IsSlice<Payload>::VALUE
                         && !HasPayloadTypeNameMember<typename Payload::ValueType>::value,
                     TypeName>;

#if !defined(__cpp_rtti) && !defined(__GXX_RTTI)
template <typename T>
constexpr auto compiler_type_name() noexcept {
    constexpr std::string_view signature = __PRETTY_FUNCTION__;
    constexpr auto marker = signature.find("T = ");
    constexpr auto end = signature.rfind(']');
    static_assert(marker != std::string_view::npos && end != std::string_view::npos,
                  "Unsupported compiler type signature");
    return signature.substr(marker + 4, end - marker - 4);
}

struct RttiFreeTypeName {
    char bytes[TypeName::capacity()] {};
};

template <typename T>
constexpr auto make_rtti_free_type_name() noexcept -> RttiFreeTypeName {
    constexpr auto name = compiler_type_name<T>();
    constexpr std::string_view prefix = "__cxx__abi__";
    constexpr std::string_view hex = "0123456789abcdef";
    constexpr auto limit = TypeName::capacity() - 1;
    static_assert(limit > prefix.size() + 17, "TypeName capacity cannot hold the type fingerprint");
    RttiFreeTypeName result;
    size_t size = 0;
    for (const auto c : prefix)
        result.bytes[size++] = c;
    uint64_t hash = 14695981039346656037ULL;
    for (const auto c : name) {
        const auto byte = static_cast<unsigned char>(c);
        hash = (hash ^ byte) * 1099511628211ULL;
    }
    // Keep the full-type fingerprint when truncating and escape non-ASCII bytes.
    // Compiler-derived names are build-local; portable schemas must provide an explicit name.
    for (const auto c : name) {
        const auto byte = static_cast<unsigned char>(c);
        if (byte >= 0x20 && byte <= 0x7e && c != '"' && c != '\\') {
            if (size == limit - 17)
                break;
            result.bytes[size++] = c;
        } else {
            if (size + 4 > limit - 17)
                break;
            result.bytes[size++] = '_';
            result.bytes[size++] = 'x';
            result.bytes[size++] = hex[byte >> 4];
            result.bytes[size++] = hex[byte & 0xf];
        }
    }
    result.bytes[size++] = '_';
    for (size_t i = 0; i < 16; ++i)
        result.bytes[size++] = hex[(hash >> ((15 - i) * 4)) & 0xf];
    return result;
}
#endif
template <typename PayloadType>
auto get_type_name_impl() -> internal::FromPayloadTypeNameSpecialization<PayloadType> {
    static_assert(!HasPayloadTypeNameMember<PayloadType>::value,
                  "A type must not define both an iox2::TypeNameSpecialization and an IOX2_TYPE_NAME "
                  "member. Choose exactly one source for the type name.");

    return *TypeName::from_utf8_null_terminated_unchecked(TypeNameSpecialization<PayloadType>::value());
}

template <typename PayloadType>
auto get_type_name_impl() -> internal::FromPayloadTypeNameMember<PayloadType> {
    return *TypeName::from_utf8_null_terminated_unchecked(PayloadType::IOX2_TYPE_NAME);
}

template <typename PayloadType>
auto get_type_name_impl() -> internal::FromStaticString<PayloadType> {
    // std::array is not available in this safety-critical context
    // NOLINTNEXTLINE
    char type_name[TypeName::capacity()] { 0 };
    // std::to_string() is not available in this safety-critical context
    // NOLINTNEXTLINE
    snprintf(&type_name[0],
             TypeName::capacity(),
             "iceoryx2_bb_container::string::static_string::StaticString<%llu>",
             static_cast<long long unsigned int>(PayloadType::capacity()));
    return *TypeName::from_utf8_null_terminated_unchecked(&type_name[0]);
}

template <typename PayloadType>
auto get_type_name_impl() -> internal::FromStaticVector<PayloadType> {
    // std::array is not available in this safety-critical context
    // NOLINTNEXTLINE
    char type_name[TypeName::capacity()] { 0 };
    // std::to_string() is not available in this safety-critical context
    // NOLINTNEXTLINE
    snprintf(&type_name[0],
             TypeName::capacity(),
             "iceoryx2_bb_container::vector::static_vec::StaticVec<%s, %llu>",
             get_type_name<typename PayloadType::ValueType>().unchecked_access().c_str(),
             static_cast<long long unsigned int>(PayloadType::capacity()));
    return *TypeName::from_utf8_null_terminated_unchecked(&type_name[0]);
}

// NOLINTBEGIN(readability-function-size) : template alternative is less readable
template <typename PayloadType>
auto get_type_name_impl() -> internal::FromNonSlice<PayloadType> {
    if (std::is_same<PayloadType, void>::value) {
        return *TypeName::from_utf8("()");
    }
    if (std::is_same<PayloadType, uint8_t>::value) {
        return *TypeName::from_utf8("u8");
    }
    if (std::is_same<PayloadType, uint16_t>::value) {
        return *TypeName::from_utf8("u16");
    }
    if (std::is_same<PayloadType, uint32_t>::value) {
        return *TypeName::from_utf8("u32");
    }
    if (std::is_same<PayloadType, uint64_t>::value) {
        return *TypeName::from_utf8("u64");
    }
    if (std::is_same<PayloadType, int8_t>::value) {
        return *TypeName::from_utf8("i8");
    }
    if (std::is_same<PayloadType, int16_t>::value) {
        return *TypeName::from_utf8("i16");
    }
    if (std::is_same<PayloadType, int32_t>::value) {
        return *TypeName::from_utf8("i32");
    }
    if (std::is_same<PayloadType, int64_t>::value) {
        return *TypeName::from_utf8("i64");
    }
    if (std::is_same<PayloadType, float>::value) {
        return *TypeName::from_utf8("f32");
    }
    if (std::is_same<PayloadType, double>::value) {
        return *TypeName::from_utf8("f64");
    }

#if defined(__cpp_rtti) || defined(__GXX_RTTI)
    // std::array is not available in this safety-critical context
    // NOLINTNEXTLINE
    char type_name[TypeName::capacity()] { 0 };
    // std::to_string() is not available in this safety-critical context
    // NOLINTNEXTLINE
    snprintf(&type_name[0],
             TypeName::capacity(),
             "__cxx__abi__%s",
             typeid(typename PayloadInfo<PayloadType>::ValueType).name());
    return *TypeName::from_utf8_null_terminated_unchecked(&type_name[0]);
#else
    constexpr auto fallback = make_rtti_free_type_name<typename PayloadInfo<PayloadType>::ValueType>();
    return *TypeName::from_utf8_null_terminated_unchecked(fallback.bytes);
#endif
}
// NOLINTEND(readability-function-size)

template <typename PayloadType>
auto get_type_name_impl() -> internal::FromSliceWithInnerPayloadTypeNameMember<PayloadType> {
    return *TypeName::from_utf8_null_terminated_unchecked(PayloadType::ValueType::IOX2_TYPE_NAME);
}

template <typename PayloadType>
auto get_type_name_impl() -> internal::FromSliceWithoutInnerPayloadTypeNameMember<PayloadType> {
    return get_type_name_impl<typename PayloadType::ValueType>();
}

template <typename PayloadType>
auto get_type_name() -> TypeName {
    return get_type_name_impl<PayloadType>();
}

} // namespace internal
} // namespace iox2

#endif
