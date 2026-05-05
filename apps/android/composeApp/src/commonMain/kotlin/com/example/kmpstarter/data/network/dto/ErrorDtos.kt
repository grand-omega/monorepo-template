package com.example.kmpstarter.data.network.dto

import kotlinx.serialization.Serializable

@Serializable
data class ErrorBodyDto(
    val code: String,
    val message: String,
    val fields: List<FieldErrorDto>? = null,
)

@Serializable
data class FieldErrorDto(
    val field: String,
    val code: String,
)
