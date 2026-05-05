package com.example.kmpstarter.domain

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class ValidatorsTest {

    @Test
    fun `email blank is rejected`() {
        assertNotNull(Validators.emailError(""))
        assertNotNull(Validators.emailError("   "))
    }

    @Test
    fun `email without at is rejected`() {
        assertNotNull(Validators.emailError("foo"))
    }

    @Test
    fun `email without dot is rejected`() {
        assertNotNull(Validators.emailError("foo@bar"))
    }

    @Test
    fun `valid email passes`() {
        assertNull(Validators.emailError("a@b.c"))
        assertNull(Validators.emailError("user.name+tag@example.co.uk"))
    }

    @Test
    fun `password blank is rejected`() {
        assertNotNull(Validators.passwordError(""))
    }

    @Test
    fun `password under min length is rejected`() {
        assertNotNull(Validators.passwordError("short"))
    }

    @Test
    fun `password at or over min length passes`() {
        assertNull(Validators.passwordError("12345678"))
        assertNull(Validators.passwordError("a-very-long-password!"))
    }

    @Test
    fun `password match catches mismatch`() {
        assertNotNull(Validators.passwordMatchError("abcd1234", "different"))
        assertNotNull(Validators.passwordMatchError("abcd1234", ""))
    }

    @Test
    fun `password match passes when equal`() {
        assertNull(Validators.passwordMatchError("abcd1234", "abcd1234"))
    }

    @Test
    fun `display name over 64 chars is rejected`() {
        assertNotNull(Validators.displayNameError("a".repeat(65)))
        assertNull(Validators.displayNameError("Alice"))
    }

    @Test
    fun `apiFieldErrorMessage maps known codes`() {
        assertEquals("Enter a valid email", Validators.apiFieldErrorMessage("email", "format"))
        assertEquals("Email is already in use", Validators.apiFieldErrorMessage("email", "taken"))
        assertEquals("Password is too short", Validators.apiFieldErrorMessage("password", "too_short"))
    }

    @Test
    fun `apiFieldErrorMessage falls back for unknown codes`() {
        val msg = Validators.apiFieldErrorMessage("foo", "weird_code")
        assertEquals("Foo: weird_code", msg)
    }
}
