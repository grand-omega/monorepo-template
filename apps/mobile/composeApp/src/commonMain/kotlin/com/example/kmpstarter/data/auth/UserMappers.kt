package com.example.kmpstarter.data.auth

import com.example.kmpstarter.data.network.dto.UserSummaryDto
import com.example.kmpstarter.domain.User

internal fun UserSummaryDto.toUser(): User =
    User(
        id = id,
        email = email,
        displayName = displayName,
        emailVerified = emailVerified,
    )
