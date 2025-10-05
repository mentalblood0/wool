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
    (users.get u.id).should eq u

    i = Wool::Users::Integration.new u.id, Wool::Users::Site::Telegram, "telegramid"
    users.add i
    (users.get Wool::Users::Site::Telegram, "telegramid").should eq u
    users.delete u.id
    (users.get u.id).should eq nil

    users.delete u.id
    (users.get u.id).should eq nil
    (users.get Wool::Users::Site::Telegram, "telegramid").should eq nil
  end

  it "can push commands to queue" do
    u = Wool::User.new (Wool::User::Name.new "name"), Wool::User::Role::User
    users.add u

    c = Wool::Command::Add.new({c: Wool::Text.new "text"})
    users.push u.id, c

    u = (users.get u.id).not_nil!
    u.queue.should eq [c]
  end
end
