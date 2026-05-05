package com.example.kmpstarter.data.auth

import com.russhwolf.settings.MapSettings
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class SettingsTokenStorageTest {

    @Test
    fun `read returns null before any save`() = runTest {
        val storage = SettingsTokenStorage(MapSettings())
        assertNull(storage.readRefreshToken())
    }

    @Test
    fun `save then read returns the same value`() = runTest {
        val storage = SettingsTokenStorage(MapSettings())
        storage.saveRefreshToken("rt-123")
        assertEquals("rt-123", storage.readRefreshToken())
    }

    @Test
    fun `clear wipes the stored token`() = runTest {
        val storage = SettingsTokenStorage(MapSettings())
        storage.saveRefreshToken("rt-123")
        storage.clear()
        assertNull(storage.readRefreshToken())
    }

    @Test
    fun `save overwrites previous value`() = runTest {
        val storage = SettingsTokenStorage(MapSettings())
        storage.saveRefreshToken("first")
        storage.saveRefreshToken("second")
        assertEquals("second", storage.readRefreshToken())
    }
}
