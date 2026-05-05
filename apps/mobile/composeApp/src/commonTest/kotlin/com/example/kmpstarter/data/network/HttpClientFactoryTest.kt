package com.example.kmpstarter.data.network

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class HttpClientFactoryTest {

    @Test
    fun `redactTokens replaces access_token, refresh_token, and Authorization`() {
        val raw = """
            POST /v1/auth/login
            Authorization: Bearer eyJABCDEFG.HIJKLMNOP.QRSTUV
            BODY: {"access_token":"abc.def.ghi","refresh_token":"rt-xyz"}
        """.trimIndent()

        val redacted = redactTokens(raw)

        assertTrue("eyJABCDEFG" !in redacted, "access bearer leaked: $redacted")
        assertTrue("abc.def.ghi" !in redacted, "access_token value leaked: $redacted")
        assertTrue("rt-xyz" !in redacted, "refresh_token value leaked: $redacted")
        assertTrue("<redacted>" in redacted)
    }

    @Test
    fun `redactTokens leaves non-token text alone`() {
        val raw = "GET /v1/me\nuser_id=u-1"
        assertEquals(raw, redactTokens(raw))
    }
}
