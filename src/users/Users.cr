require "yaml"

require "trove"
require "woollib/common"

module Wool
  class Users
    mserializable

    getter chest : Trove::Chest

    def add(u : User)
      raise Exception.new "User #{u.to_json} already exists" if @chest.has_key! u.id, "type"
      @chest.set u.id, "", Wool.to_tj u
    end

    def get(un : User::Name)
      Wool.from_tj User, (@chest.get (User.new un).id).not_nil! rescue nil
    end

    def delete(un : User::Name)
      @chest.delete (User.new un).id
    end

    def add(i : Integration)
      raise Exception.new "Integration #{i.to_json} already exists" if @chest.has_key! i.id, "type"
      @chest.set i.id, "", Wool.to_tj i
    end

    def delete(i : Integration)
      @chest.delete i.id
    end

    def add(un : User::Name, c : Wool::Command)
      @chest.push (User.new un).id, "user.queue", Wool.to_tj c
    end
  end
end
