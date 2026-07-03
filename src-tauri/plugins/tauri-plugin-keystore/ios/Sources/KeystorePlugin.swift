import SwiftRs
import Tauri
import UIKit
import WebKit

import LocalAuthentication

class StoreRequest: Decodable {
  let service: String
  let user: String
  let value: String
  let biometric: Bool?
}

class ItemRequest: Decodable {
  let service: String
  let user: String
}

class KeystorePlugin: Plugin {
  private func keychainError(_ status: OSStatus) -> NSError {
    let message = SecCopyErrorMessageString(status, nil) as String? ?? "OSStatus \(status)"
    return NSError(domain: NSOSStatusErrorDomain, code: Int(status), userInfo: [NSLocalizedDescriptionKey: message])
  }

  @objc public func store(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(StoreRequest.self)

    guard let secretData = args.value.data(using: .utf8) else {
        throw NSError(domain: "StoreErrorDomain", code: -1, userInfo: [NSLocalizedDescriptionKey: "Invalid secret string"])
    }

    var query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: args.service,
        kSecAttrAccount as String: args.user,
        kSecValueData as String: secretData
    ]

    if args.biometric ?? true {
        var error: Unmanaged<CFError>?
        guard let accessControl = SecAccessControlCreateWithFlags(
            nil,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            .userPresence,
            &error
        ) else {
            throw error?.takeRetainedValue() ?? NSError(domain: "app.passero.keystore", code: -1, userInfo: nil)
        }
        query[kSecAttrAccessControl as String] = accessControl
    } else {
        query[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    }

    var status = SecItemAdd(query as CFDictionary, nil)
    if status == errSecDuplicateItem {
        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: args.service,
            kSecAttrAccount as String: args.user
        ]
        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw keychainError(deleteStatus)
        }
        status = SecItemAdd(query as CFDictionary, nil)
    }
    guard status == errSecSuccess else {
        throw keychainError(status)
    }

    invoke.resolve()
  }

  @objc public func retrieve(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(ItemRequest.self)

      let context = LAContext()
      context.localizedReason = "Access your Passero secrets"

      let query: [String: Any] = [
          kSecClass as String: kSecClassGenericPassword,
          kSecAttrService as String: args.service,
          kSecAttrAccount as String: args.user,
          kSecReturnData as String: true,
          kSecMatchLimit as String: kSecMatchLimitOne,
          kSecUseAuthenticationContext as String: context
      ]

      var item: CFTypeRef?
      let status = SecItemCopyMatching(query as CFDictionary, &item)

      if status == errSecItemNotFound {
          invoke.resolve(["value": NSNull()])
          return
      }

      guard status == errSecSuccess else {
          throw keychainError(status)
      }

      guard let data = item as? Data,
            let secret = String(data: data, encoding: .utf8) else {
          throw NSError(domain: "app.passero.keystore", code: -1, userInfo: [NSLocalizedDescriptionKey: "Unable to decode secret"])
      }

      invoke.resolve(["value": secret])
  }

  @objc public func remove(_ invoke: Invoke) throws {
      let args = try invoke.parseArgs(ItemRequest.self)

      let query: [String: Any] = [
          kSecClass as String: kSecClassGenericPassword,
          kSecAttrService as String: args.service,
          kSecAttrAccount as String: args.user
      ]

      let status = SecItemDelete(query as CFDictionary)

      guard status == errSecSuccess || status == errSecItemNotFound else {
          throw keychainError(status)
      }

      invoke.resolve()
  }
}

@_cdecl("init_plugin_keystore")
func initPlugin() -> Plugin {
  return KeystorePlugin()
}
