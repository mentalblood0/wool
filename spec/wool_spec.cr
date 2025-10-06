require "spec"

require "../src/users/User"
require "../src/users/Users"
require "../src/Service"
require "../src/Command"
require "../src/users/Integration"

alias Config = {service: Wool::Service}

describe Wool do
  config = Config.from_yaml File.read "spec/config.yml"

  users = config[:service].users
  describe Wool::Users do
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

      users.delete u.id
    end
  end

  service = config[:service]
  describe Wool::Service do
    it "can answer to commands" do
      uu = Wool::User.new (Wool::User::Name.new "user"), Wool::User::Role::User
      um = Wool::User.new (Wool::User::Name.new "moderator"), Wool::User::Role::Moderator
      users.add uu
      users.add um

      iuu = Wool::Users::Integration.new uu.id, Wool::Users::Site::Telegram, "iuu"
      users.add iuu
      ium = Wool::Users::Integration.new um.id, Wool::Users::Site::Telegram, "ium"
      users.add ium

      un = Wool::User.new (Wool::User::Name.new "new"), Wool::User::Role::User
      c = Wool::Command::AddUser.new({u: un})
      (service.answer Wool::Users::Site::Telegram, "iuu", c).should eq Wool::Service::Error::OperationNotPermitted
      (service.answer Wool::Users::Site::Telegram, "ium", c).should eq un.id
    end
  end
end
