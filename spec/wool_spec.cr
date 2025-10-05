require "spec"

require "../src/users/User"
require "../src/users/Users"
require "../src/users/Integration"

alias Config = {users: Wool::Users}

describe Wool do
  config = Config.from_yaml File.read "spec/config.yml"
  users = config[:users]

  it "can add/delete users and integrations" do
    u = Wool::User.new (Wool::User::Name.new "name"), Wool::User::Role::User
    users.add u
    (users.get u.name).should eq u

    i = Wool::Users::Integration.new u.id, Wool::Users::Site::Telegram, "telegramid"
    users.add i
    (users.get Wool::Users::Site::Telegram, "telegramid").should eq u
    users.delete u.name
    (users.get u.name).should eq nil

    users.delete u.name
    (users.get u.name).should eq nil
    (users.get Wool::Users::Site::Telegram, "telegramid").should eq nil
  end
end
