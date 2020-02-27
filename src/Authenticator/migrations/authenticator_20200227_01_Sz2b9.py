"""

"""

from yoyo import step

__depends__ = {'authenticator_20190529_01_8bpUj-empty-uneeded-provider-images'}

steps = [
    step("ALTER TABLE providers ADD COLUMN period INTEGER DEFAULT 30;"),
    step("ALTER TABLE providers ADD COLUMN type INTEGER DEFAULT 'TOTP';")
]
