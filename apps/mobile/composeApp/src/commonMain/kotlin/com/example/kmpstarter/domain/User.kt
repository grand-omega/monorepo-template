package com.example.kmpstarter.domain

/** UI-facing user model. Decouples screens from network DTOs. */
data class User(
    val id: String,
    val email: String,
    val displayName: String?,
    val emailVerified: Boolean,
)
