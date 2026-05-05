package com.example.kmpstarter.data.network.dto

import com.example.kmpstarter.data.network.ApiJson
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DtoSerializationTest {

    @Test
    fun `TokenPair decodes the OpenAPI shape`() {
        val json = """
            {
              "access_token": "eyJ.access",
              "refresh_token": "rt.value",
              "token_type": "Bearer",
              "access_expires_in": 900,
              "user": {
                "id": "11111111-2222-3333-4444-555555555555",
                "email": "u@example.com",
                "display_name": "Alice",
                "email_verified": true
              }
            }
        """.trimIndent()

        val pair = ApiJson.decodeFromString<TokenPairDto>(json)

        assertEquals("eyJ.access", pair.accessToken)
        assertEquals("rt.value", pair.refreshToken)
        assertEquals("Bearer", pair.tokenType)
        assertEquals(900L, pair.accessExpiresInSeconds)
        assertEquals("Alice", pair.user.displayName)
        assertTrue(pair.user.emailVerified)
    }

    @Test
    fun `UserSummary decodes when display_name missing`() {
        val json = """
            { "id": "abc", "email": "x@y.z", "email_verified": false }
        """.trimIndent()

        val u = ApiJson.decodeFromString<UserSummaryDto>(json)
        assertEquals("abc", u.id)
        assertNull(u.displayName)
        assertEquals(false, u.emailVerified)
    }

    @Test
    fun `RegisterRequest with null display_name omits it from JSON`() {
        val req = RegisterRequestDto(email = "a@b.c", password = "secret123!", displayName = null)
        val out = ApiJson.encodeToString(RegisterRequestDto.serializer(), req)
        assertTrue("display_name" !in out, "display_name should be omitted; got: $out")
        assertTrue("\"email\":\"a@b.c\"" in out)
    }

    @Test
    fun `ErrorBody parses field errors`() {
        val json = """
            {
              "code": "validation_failed",
              "message": "Validation failed",
              "fields": [
                { "field": "email", "code": "format" },
                { "field": "password", "code": "too_short" }
              ]
            }
        """.trimIndent()

        val err = ApiJson.decodeFromString<ErrorBodyDto>(json)
        assertEquals("validation_failed", err.code)
        assertEquals(2, err.fields?.size)
        assertEquals("email", err.fields?.get(0)?.field)
        assertEquals("too_short", err.fields?.get(1)?.code)
    }

    @Test
    fun `ErrorBody parses without fields`() {
        val json = """{ "code": "rate_limited", "message": "Slow down" }"""
        val err = ApiJson.decodeFromString<ErrorBodyDto>(json)
        assertNull(err.fields)
        assertEquals("Slow down", err.message)
    }

    @Test
    fun `unknown JSON keys are ignored`() {
        // The server may add fields we don't model — we shouldn't crash.
        val json = """
            { "id": "x", "email": "a@b.c", "email_verified": true, "future_flag": 42 }
        """.trimIndent()
        val u = ApiJson.decodeFromString<UserSummaryDto>(json)
        assertEquals("x", u.id)
    }
}
